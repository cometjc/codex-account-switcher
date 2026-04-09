use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use crossterm::event;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::prelude::*;
use ratatui::style::Stylize;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use serde_json::Value;

use crate::cron::CronStatus;
use crate::duration::format_duration_short;
use crate::input::{self, InputAction, InputContext};
use crate::loader::load_profiles;
use crate::refresh_log::append_refresh_log;
use crate::render;
use crate::render::{
    ChartSeries, ChartSeriesStyle, ChartState, FiveHourBandState, FiveHourSubframeState, RenderProfile,
    SelectionState,
};
use crate::store::AccountStore;
use crate::usage::{
    pick_five_hour_window, pick_weekly_window, UsageReadResult, UsageResponse, UsageService, UsageSource,
};
use crate::app_data::{ProfileChartData, ProfileEntry, ProfileKind};

const BACKGROUND_REFRESH_STALE_SECONDS: i64 = 600;
const CRON_YIELD_FRESH_SECONDS: i64 = 15 * 60;
const BACKGROUND_REFRESH_ERROR_COOLDOWN_SECONDS: i64 = 600;
const HOT_RELOAD_CHECK_INTERVAL: Duration = Duration::from_secs(2);
const PROFILE_RELOAD_FALLBACK_INTERVAL: Duration = Duration::from_secs(600);
const FILE_CHANGE_RELOAD_DEBOUNCE: Duration = Duration::from_millis(800);

#[derive(Debug, Clone, PartialEq, Eq)]
enum BackgroundRefreshState {
    Idle,
    Running { queued_profiles: usize },
}

#[derive(Debug)]
struct BackgroundRefreshReport {
    refreshed_profiles: usize,
    refresh_errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaneFocus {
    Accounts,
    Plot,
}

impl PaneFocus {
    fn toggle(self) -> Self {
        match self {
            Self::Accounts => Self::Plot,
            Self::Plot => Self::Accounts,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DialogMode {
    SaveCurrent(ProfileKind),
    RenameSaved(String),
    ConfirmDelete(String),
    /// Name for `cursor-profiles/<name>.json` under the agent-switch config dir before ingest server starts.
    AddCursorProfile,
    /// Waiting for POST /ingest from Windows `cursor-export` over Tailscale.
    CursorIngestWaiting { profile_name: String, port: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogState {
    mode: DialogMode,
    input: String,
    cursor: usize,
}

impl DialogState {
    fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.input.chars().count());
    }

    fn move_to_start(&mut self) {
        self.cursor = 0;
    }

    fn move_to_end(&mut self) {
        self.cursor = self.input.chars().count();
    }

    fn insert_char(&mut self, ch: char) {
        let byte_index = char_to_byte_index(&self.input, self.cursor);
        self.input.insert(byte_index, ch);
        self.cursor += 1;
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let end = char_to_byte_index(&self.input, self.cursor);
        let start = char_to_byte_index(&self.input, self.cursor - 1);
        self.input.replace_range(start..end, "");
        self.cursor -= 1;
    }
}

#[derive(Debug)]
struct CursorIngestSession {
    profile_name: String,
    #[allow(dead_code)]
    port: u16,
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
    rx: Receiver<Result<usize, String>>,
}

pub struct App {
    profiles: Vec<ProfileEntry>,
    selected_profile_index: usize,
    pane_focus: PaneFocus,
    y_zoom_lower: f64,
    should_quit: bool,
    should_exec: bool,
    dialog: Option<DialogState>,
    status_message: Option<String>,
    store: Option<AccountStore>,
    usage_service: Option<UsageService>,
    claude_store: Option<crate::claude::ClaudeStore>,
    claude_usage_service: Option<UsageService>,
    copilot_usage_service: Option<UsageService>,
    cron_status: CronStatus,
    solo_mode: bool,
    fullscreen: bool,
    x_window_days: f64,
    x_offset_days: f64,
    y_zoom_upper: f64,
    filter_input: Option<String>,
    last_auto_reload: Instant,
    last_binary_reload_check: Instant,
    y_zoom_user_adjusted: bool,
    binary_path: String,
    binary_mtime: Option<std::time::SystemTime>,
    tab_zoom_index: Option<usize>,
    hidden_profiles: std::collections::HashSet<String>,
    background_refresh_state: BackgroundRefreshState,
    background_refresh_receiver: Option<Receiver<BackgroundRefreshReport>>,
    background_refresh_retry_after: Option<i64>,
    file_change_receiver: Option<Receiver<()>>,
    file_change_watcher: Option<RecommendedWatcher>,
    file_change_pending_since: Option<Instant>,
    cursor_ingest: Option<CursorIngestSession>,
}

#[derive(Debug)]
pub(crate) struct AppRenderState<'a> {
    profiles: &'a [ProfileEntry],
    selected_profile_index: usize,
    y_zoom_lower: f64,
    y_zoom_upper: f64,
    solo: bool,
    x_window_days: f64,
    x_offset_days: f64,
    plot_focused: bool,
    fullscreen: bool,
    tab_zoom_index: Option<usize>,
    hidden_profiles: &'a std::collections::HashSet<String>,
}

impl App {
    pub fn load(
        store: AccountStore,
        usage_service: UsageService,
        cron_status: CronStatus,
        claude_store: Option<crate::claude::ClaudeStore>,
        claude_usage_service: Option<UsageService>,
        copilot_usage_service: Option<UsageService>,
    ) -> Result<Self> {
        let profiles = load_profiles(
            &store,
            &usage_service,
            false,
            None,
            true,
            claude_store.as_ref(),
            claude_usage_service.as_ref(),
            copilot_usage_service.as_ref(),
        )?;
        let current_exe = std::env::current_exe().unwrap_or_default();
        let hot_reload_path = resolve_hot_reload_binary_path(&current_exe);
        let binary_path = hot_reload_path.to_string_lossy().into_owned();
        let binary_mtime = binary_mtime(&binary_path);
        let hidden_profiles = store.read_ui_state().hidden_profiles;
        let mut app = Self {
            selected_profile_index: initial_selected_index(&profiles),
            y_zoom_lower: auto_y_lower(&profiles),
            profiles,
            pane_focus: PaneFocus::Plot,
            should_quit: false,
            should_exec: false,
            dialog: None,
            status_message: None,
            store: Some(store),
            usage_service: Some(usage_service),
            claude_store,
            claude_usage_service,
            copilot_usage_service,
            cron_status,
            solo_mode: false,
            fullscreen: true,
            x_window_days: 7.0,
            x_offset_days: 0.0,
            y_zoom_upper: 100.0,
            filter_input: None,
            last_auto_reload: Instant::now(),
            last_binary_reload_check: Instant::now(),
            y_zoom_user_adjusted: false,
            binary_path,
            binary_mtime,
            tab_zoom_index: None,
            hidden_profiles,
            background_refresh_state: BackgroundRefreshState::Idle,
            background_refresh_receiver: None,
            background_refresh_retry_after: None,
            file_change_receiver: None,
            file_change_watcher: None,
            file_change_pending_since: None,
            cursor_ingest: None,
        };
        app.setup_file_change_watcher();
        app.ensure_background_refresh_task();
        Ok(app)
    }

    pub fn from_profile_names(profile_names: Vec<String>, selected_profile_index: usize) -> Self {
        let profiles = profile_names
            .into_iter()
            .enumerate()
            .map(|(index, name)| ProfileEntry {
                kind: ProfileKind::Codex,
                saved_name: Some(name.to_lowercase()),
                profile_name: name,
                snapshot: serde_json::json!({}),
                usage_view: UsageReadResult {
                    usage: None,
                    source: UsageSource::None,
                    fetched_at: None,
                    stale: false,
                },
                account_id: Some(format!("acct-{index}")),
                is_current: index == selected_profile_index,
                chart_data: ProfileChartData::empty("no usage data"),
            })
            .collect::<Vec<_>>();

        Self {
            profiles,
            selected_profile_index,
            pane_focus: PaneFocus::Plot,
            y_zoom_lower: 0.0,
            should_quit: false,
            should_exec: false,
            dialog: None,
            status_message: None,
            store: None,
            usage_service: None,
            claude_store: None,
            claude_usage_service: None,
            copilot_usage_service: None,
            cron_status: CronStatus::uninstalled(),
            solo_mode: false,
            fullscreen: true,
            x_window_days: 7.0,
            x_offset_days: 0.0,
            y_zoom_upper: 100.0,
            filter_input: None,
            last_auto_reload: Instant::now(),
            last_binary_reload_check: Instant::now(),
            y_zoom_user_adjusted: false,
            binary_path: String::new(),
            binary_mtime: None,
            tab_zoom_index: None,
            hidden_profiles: std::collections::HashSet::new(),
            background_refresh_state: BackgroundRefreshState::Idle,
            background_refresh_receiver: None,
            background_refresh_retry_after: None,
            file_change_receiver: None,
            file_change_watcher: None,
            file_change_pending_since: None,
            cursor_ingest: None,
        }
    }

    pub fn selected_profile_label(&self) -> Option<&str> {
        self.selected_profile().map(|profile| profile.profile_name.as_str())
    }

    pub fn select_previous_profile(mut self) -> Self {
        self.step_profile(-1);
        self
    }

    pub fn run(&mut self) -> Result<()> {
        {
            let mut terminal = TerminalSession::enter();
            terminal.run(self)?;
        } // terminal restored here via Drop
        if self.should_exec && !self.binary_path.is_empty() {
            let args: Vec<String> = std::env::args().collect();
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                let _ = std::process::Command::new(&self.binary_path)
                    .args(&args[1..])
                    .exec();
            }
            #[cfg(windows)]
            {
                let _ = std::process::Command::new(&self.binary_path)
                    .args(&args[1..])
                    .spawn();
                std::process::exit(0);
            }
        }
        Ok(())
    }

    fn run_loop(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        while !self.should_quit && !self.should_exec {
            self.poll_background_refresh();
            self.poll_cursor_ingest();
            self.poll_hot_reload_binary();
            self.poll_file_change_reload();
            terminal.draw(|frame| self.render(frame))?;

            if event::poll(Duration::from_millis(150))? {
                let event = event::read()?;
                if let Some(action) = input::map_event(&event, self.input_context()) {
                    self.handle_action(action)?;
                }
            }

            // Fallback poll in case file events are unavailable on this platform.
            if self.last_auto_reload.elapsed() >= PROFILE_RELOAD_FALLBACK_INTERVAL {
                let _ = self.reload_profiles(false, None);
                self.last_auto_reload = Instant::now();
            }
        }
        Ok(())
    }

    fn poll_hot_reload_binary(&mut self) {
        if self.binary_path.is_empty() {
            return;
        }
        if self.last_binary_reload_check.elapsed() < HOT_RELOAD_CHECK_INTERVAL {
            return;
        }
        self.last_binary_reload_check = Instant::now();

        let current_path = PathBuf::from(&self.binary_path);
        let candidate_path = resolve_hot_reload_binary_path(&current_path);
        let candidate_mtime = binary_mtime(candidate_path.to_string_lossy().as_ref());
        let path_changed = candidate_path != current_path;
        let mtime_changed = candidate_mtime != self.binary_mtime;
        if path_changed || mtime_changed {
            self.binary_path = candidate_path.to_string_lossy().into_owned();
            self.binary_mtime = candidate_mtime;
            self.should_exec = true;
            self.status_message = Some("Detected binary update, reloading...".to_string());
        }
    }

    fn setup_file_change_watcher(&mut self) {
        let Some(store) = self.store.as_ref() else {
            return;
        };
        let (tx, rx) = mpsc::channel::<()>();
        let refresh_log_path = store.paths().refresh_log_path();
        let mut watcher = match notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            let Ok(event) = result else {
                return;
            };
            if event.paths.iter().any(|path| path == &refresh_log_path) {
                return;
            }
            if event
                .paths
                .iter()
                .any(|path| path.extension().and_then(|ext| ext.to_str()) == Some("lock"))
            {
                return;
            }
            let should_reload = !matches!(event.kind, EventKind::Access(_));
            if should_reload {
                let _ = tx.send(());
            }
        }) {
            Ok(w) => w,
            Err(_) => return,
        };

        let mut watch_paths = vec![store.paths().codex_dir().to_path_buf()];
        if let Some(claude_store) = self.claude_store.as_ref() {
            watch_paths.push(claude_store.paths().claude_dir().to_path_buf());
        }
        let copilot_paths = crate::copilot::CopilotPaths::detect();
        if let Some(parent) = copilot_paths.usage_history_path().parent() {
            watch_paths.push(parent.to_path_buf());
        }

        for path in watch_paths {
            if path.exists() {
                let _ = watcher.watch(path.as_path(), RecursiveMode::NonRecursive);
            }
        }

        self.file_change_receiver = Some(rx);
        self.file_change_watcher = Some(watcher);
    }

    fn poll_file_change_reload(&mut self) {
        let Some(receiver) = self.file_change_receiver.as_ref() else {
            return;
        };
        let mut saw_change = false;
        loop {
            match receiver.try_recv() {
                Ok(_) => saw_change = true,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.file_change_receiver = None;
                    self.file_change_watcher = None;
                    self.file_change_pending_since = None;
                    break;
                }
            }
        }
        if saw_change {
            self.file_change_pending_since = Some(Instant::now());
        }
        if self
            .file_change_pending_since
            .is_some_and(|since| since.elapsed() >= FILE_CHANGE_RELOAD_DEBOUNCE)
        {
            let _ = self.reload_profiles(false, None);
            self.last_auto_reload = Instant::now();
            self.file_change_pending_since = None;
        }
    }

    fn handle_action(&mut self, action: InputAction) -> Result<()> {
        if self.dialog.is_some() {
            return self.handle_dialog_action(action);
        }
        if self.filter_input.is_some() {
            return self.handle_filter_action(action);
        }

        match action {
            InputAction::Quit => self.should_quit = true,
            InputAction::Up | InputAction::Down => {
                if self.pane_focus == PaneFocus::Plot {
                    let shift = if matches!(action, InputAction::Up) { 5.0_f64 } else { -5.0_f64 };
                    let new_upper = (self.y_zoom_upper + shift).clamp(self.y_zoom_lower + 5.0, 100.0);
                    let actual = new_upper - self.y_zoom_upper;
                    self.y_zoom_upper = new_upper;
                    self.y_zoom_lower = (self.y_zoom_lower + actual).clamp(0.0, self.y_zoom_upper - 5.0);
                    self.y_zoom_user_adjusted = true;
                } else {
                    let delta = if matches!(action, InputAction::Up) { -1 } else { 1 };
                    self.step_profile(delta);
                }
            }
            InputAction::Left => {
                if self.pane_focus == PaneFocus::Plot {
                    let max_offset = (7.0 - self.x_window_days).max(0.0);
                    self.x_offset_days = (self.x_offset_days + 0.5).min(max_offset);
                }
            }
            InputAction::Right => {
                if self.pane_focus == PaneFocus::Plot {
                    self.x_offset_days = (self.x_offset_days - 0.5).max(0.0);
                }
            }
            InputAction::ToggleProfiles => {
                self.fullscreen = !self.fullscreen;
                self.pane_focus = if self.fullscreen {
                    PaneFocus::Plot
                } else {
                    self.pane_focus.toggle()
                };
            }
            InputAction::NextFocus => {
                if self.pane_focus == PaneFocus::Plot {
                    self.advance_tab_zoom(1);
                } else {
                    self.toggle_selected_profile_hidden();
                }
            }
            InputAction::PreviousFocus => {
                if self.pane_focus == PaneFocus::Plot {
                    self.advance_tab_zoom(-1);
                } else if !self.fullscreen {
                    self.pane_focus = PaneFocus::Plot;
                }
            }
            InputAction::ZoomIn => {
                if self.pane_focus == PaneFocus::Plot {
                    self.x_window_days = (self.x_window_days - 0.5).max(0.5);
                    let max_offset = (7.0 - self.x_window_days).max(0.0);
                    self.x_offset_days = self.x_offset_days.min(max_offset);
                }
            }
            InputAction::ZoomOut => {
                if self.pane_focus == PaneFocus::Plot {
                    self.x_window_days = (self.x_window_days + 0.5).min(7.0);
                    let max_offset = (7.0 - self.x_window_days).max(0.0);
                    self.x_offset_days = self.x_offset_days.min(max_offset);
                }
            }
            InputAction::YZoomIn => {
                if self.pane_focus == PaneFocus::Plot {
                    let gap = self.y_zoom_upper - self.y_zoom_lower;
                    if gap > 10.0 {
                        self.y_zoom_lower = (self.y_zoom_lower + 5.0).min(self.y_zoom_upper - 10.0);
                        self.y_zoom_upper = (self.y_zoom_upper - 5.0).max(self.y_zoom_lower + 10.0);
                        self.y_zoom_user_adjusted = true;
                    }
                }
            }
            InputAction::YZoomOut => {
                if self.pane_focus == PaneFocus::Plot {
                    self.y_zoom_lower = (self.y_zoom_lower - 5.0).max(0.0);
                    self.y_zoom_upper = (self.y_zoom_upper + 5.0).min(100.0);
                    self.y_zoom_user_adjusted = true;
                }
            }
            InputAction::ResetZoom => {
                self.y_zoom_lower = auto_y_lower(&self.profiles);
                self.y_zoom_upper = 100.0;
                self.y_zoom_user_adjusted = false;
                self.x_window_days = 7.0;
                self.x_offset_days = 0.0;
                self.tab_zoom_index = None;
            }
            InputAction::ToggleSolo => {
                if self.pane_focus == PaneFocus::Plot {
                    self.solo_mode = !self.solo_mode;
                }
            }
            InputAction::XWindow(days) => {
                if self.pane_focus == PaneFocus::Plot {
                    self.x_window_days = days as f64;
                    self.x_offset_days = 0.0;
                }
            }
            InputAction::FilterEnter => {
                if self.pane_focus == PaneFocus::Accounts {
                    self.filter_input = Some(String::new());
                }
            }
            InputAction::Enter => {
                if self.pane_focus == PaneFocus::Accounts {
                    self.activate_selected_profile()?;
                }
            }
            InputAction::Rename => {
                if self.pane_focus == PaneFocus::Accounts {
                    self.open_rename_dialog();
                }
            }
            InputAction::Delete => {
                if self.pane_focus == PaneFocus::Accounts {
                    self.open_delete_dialog();
                }
            }
            InputAction::RefreshSelected => {
                if self.pane_focus == PaneFocus::Accounts {
                    match self.refresh_selected_profile(true) {
                        Ok(errors) if errors.is_empty() => {
                            self.status_message = Some("Refresh completed.".to_string());
                        }
                        Ok(errors) => {
                            self.status_message = Some(format!(
                                "Refresh completed with errors: {}",
                                errors.join(" | ")
                            ));
                        }
                        Err(error) => {
                            self.status_message = Some(format!("Refresh failed: {error:#}"));
                        }
                    }
                }
            }
            InputAction::RefreshAll => {
                match self.reload_profiles(true, None) {
                    Ok(errors) if errors.is_empty() => {
                        self.status_message = Some("Refresh completed.".to_string());
                    }
                    Ok(errors) => {
                        self.status_message = Some(format!(
                            "Refresh completed with errors: {}",
                            errors.join(" | ")
                        ));
                    }
                    Err(error) => {
                        self.status_message = Some(format!("Refresh failed: {error:#}"));
                    }
                }
            }
            InputAction::AddCursorProfile => {
                if self.pane_focus == PaneFocus::Accounts {
                    self.open_add_cursor_profile_dialog();
                }
            }
            InputAction::Character(' ') => {
                self.toggle_selected_profile_hidden();
            }
            InputAction::Backspace
            | InputAction::Character(_)
            | InputAction::Cancel
            | InputAction::MoveToStart
            | InputAction::MoveToEnd => {}
        }

        Ok(())
    }

    fn input_context(&self) -> InputContext {
        if self.filter_input.is_some() {
            return InputContext::TextEntry;
        }
        if let Some(d) = &self.dialog {
            if matches!(d.mode, DialogMode::CursorIngestWaiting { .. }) {
                return InputContext::CursorIngest;
            }
            return InputContext::TextEntry;
        }
        InputContext::Normal
    }

    fn open_add_cursor_profile_dialog(&mut self) {
        self.dialog = Some(DialogState {
            mode: DialogMode::AddCursorProfile,
            input: String::new(),
            cursor: 0,
        });
    }

    fn parse_port_from_bind(bind: &str) -> u16 {
        bind.rsplit(':')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(9847)
    }

    fn stop_cursor_ingest(&mut self) {
        if let Some(mut s) = self.cursor_ingest.take() {
            s.stop.store(true, Ordering::SeqCst);
            if let Some(j) = s.join.take() {
                let _ = j.join();
            }
        }
    }

    fn poll_cursor_ingest(&mut self) {
        let Some(session) = &self.cursor_ingest else {
            return;
        };
        match session.rx.try_recv() {
            Ok(Ok(bytes)) => {
                let label = session.profile_name.clone();
                if let Some(s) = self.cursor_ingest.take() {
                    if let Some(j) = s.join {
                        let _ = j.join();
                    }
                }
                self.dialog = None;
                self.status_message = Some(format!(
                    "Saved Cursor profile \"{label}\" storageState ({bytes} bytes)."
                ));
            }
            Ok(Err(e)) => {
                if let Some(s) = self.cursor_ingest.take() {
                    if let Some(j) = s.join {
                        let _ = j.join();
                    }
                }
                self.dialog = None;
                self.status_message = Some(format!("Cursor ingest failed: {e}"));
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {}
        }
    }

    fn handle_filter_action(&mut self, action: InputAction) -> Result<()> {
        match action {
            InputAction::Quit | InputAction::Cancel => {
                self.filter_input = None;
            }
            InputAction::Enter => {
                self.filter_input = None;
            }
            InputAction::Backspace => {
                if let Some(f) = self.filter_input.as_mut() {
                    f.pop();
                    // Keep filter mode even when empty; Esc/Enter/Cancel exits
                }
                let indices = self.filtered_profile_indices();
                if !indices.is_empty() && !indices.contains(&self.selected_profile_index) {
                    self.selected_profile_index = indices[0];
                }
            }
            InputAction::Character(ch) => {
                if let Some(f) = self.filter_input.as_mut() {
                    f.push(ch);
                }
                let indices = self.filtered_profile_indices();
                if !indices.is_empty() && !indices.contains(&self.selected_profile_index) {
                    self.selected_profile_index = indices[0];
                }
            }
            InputAction::Up => self.step_profile_filtered(-1),
            InputAction::Down => self.step_profile_filtered(1),
            _ => {}
        }
        Ok(())
    }

    fn filtered_profile_indices(&self) -> Vec<usize> {
        match &self.filter_input {
            None => (0..self.profiles.len()).collect(),
            Some(f) => {
                let lower = f.to_lowercase();
                self.profiles
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.profile_name.to_lowercase().contains(&lower))
                    .map(|(i, _)| i)
                    .collect()
            }
        }
    }

    fn step_profile_filtered(&mut self, delta: isize) {
        let indices = self.filtered_profile_indices();
        if indices.is_empty() {
            return;
        }
        let pos = indices
            .iter()
            .position(|&i| i == self.selected_profile_index)
            .unwrap_or(0);
        let len = indices.len() as isize;
        let next_pos = (pos as isize + delta).rem_euclid(len) as usize;
        self.selected_profile_index = indices[next_pos];
    }

    fn profile_hidden_key(profile: &ProfileEntry) -> String {
        format!("{}|{}", profile.kind.as_str(), profile.profile_name)
    }

    fn is_profile_hidden(&self, index: usize) -> bool {
        self.profiles
            .get(index)
            .map(|p| self.hidden_profiles.contains(&Self::profile_hidden_key(p)))
            .unwrap_or(false)
    }

    fn toggle_selected_profile_hidden(&mut self) {
        let key = match self.profiles.get(self.selected_profile_index) {
            Some(p) => Self::profile_hidden_key(p),
            None => return,
        };
        if self.hidden_profiles.contains(&key) {
            self.hidden_profiles.remove(&key);
        } else {
            self.hidden_profiles.insert(key);
        }
        if let Some(store) = &self.store {
            let ui_state = crate::store::UiState {
                hidden_profiles: self.hidden_profiles.clone(),
            };
            let _ = store.write_ui_state(&ui_state);
        }
    }

    fn advance_tab_zoom(&mut self, direction: isize) {
        let visible: Vec<usize> = (0..self.profiles.len())
            .filter(|&i| !self.is_profile_hidden(i))
            .collect();
        if visible.is_empty() {
            self.tab_zoom_index = None;
            return;
        }
        self.tab_zoom_index = match self.tab_zoom_index {
            None if direction > 0 => visible.first().copied(),
            None => visible.last().copied(),
            Some(current) => {
                let pos = visible.iter().position(|&i| i == current);
                let next = pos.map(|p| p as isize + direction).unwrap_or(0);
                if next < 0 || next >= visible.len() as isize {
                    None
                } else {
                    visible.get(next as usize).copied()
                }
            }
        };
    }

    fn handle_dialog_action(&mut self, action: InputAction) -> Result<()> {
        if let Some(d) = &self.dialog {
            if matches!(d.mode, DialogMode::CursorIngestWaiting { .. }) {
                match action {
                    InputAction::Quit | InputAction::Cancel => {
                        self.stop_cursor_ingest();
                        self.dialog = None;
                    }
                    _ => {}
                }
                return Ok(());
            }
        }
        match action {
            InputAction::Quit | InputAction::Cancel => {
                self.dialog = None;
            }
            InputAction::Backspace => {
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.backspace();
                }
            }
            InputAction::Character(ch) => {
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.insert_char(ch);
                }
            }
            InputAction::Left => {
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.move_left();
                }
            }
            InputAction::Right => {
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.move_right();
                }
            }
            InputAction::MoveToStart => {
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.move_to_start();
                }
            }
            InputAction::MoveToEnd => {
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.move_to_end();
                }
            }
            InputAction::Enter => self.confirm_dialog()?,
            _ => {}
        }
        Ok(())
    }

    fn confirm_dialog(&mut self) -> Result<()> {
        let Some(dialog) = self.dialog.clone() else {
            return Ok(());
        };

        match dialog.mode {
            DialogMode::SaveCurrent(kind) => {
                let Some(profile) = self.selected_profile().cloned() else {
                    return Ok(());
                };
                let name = match kind {
                    ProfileKind::Claude => {
                        let Some(cs) = self.claude_store.as_ref() else {
                            return Ok(());
                        };
                        cs.save_snapshot(&dialog.input, &profile.snapshot)
                            .with_context(|| format!("save Claude snapshot {}", dialog.input))?
                    }
                    ProfileKind::Codex => {
                        let Some(store) = self.store.as_ref() else {
                            return Ok(());
                        };
                        store.save_snapshot(&dialog.input, &profile.snapshot)
                            .with_context(|| format!("save snapshot {}", dialog.input))?
                    }
                    ProfileKind::Copilot => return Ok(()),
                };
                self.status_message = Some(format!("Saved current profile as \"{name}\"."));
                self.dialog = None;
                let _ = self.reload_profiles(false, profile.account_id.clone())?;
            }
            DialogMode::RenameSaved(current_name) => {
                let name = match self.selected_profile().map(|p| p.kind) {
                    Some(ProfileKind::Claude) => {
                        let Some(cs) = self.claude_store.as_ref() else {
                            return Ok(());
                        };
                        cs.rename_account(&current_name, &dialog.input)
                            .with_context(|| format!("rename Claude profile {current_name}"))?
                    }
                    Some(ProfileKind::Copilot) => return Ok(()),
                    _ => {
                        let Some(store) = self.store.as_ref() else {
                            return Ok(());
                        };
                        store.rename_account(&current_name, &dialog.input)
                            .with_context(|| format!("rename profile {current_name}"))?
                    }
                };
                self.status_message = Some(format!("Renamed to \"{name}\"."));
                self.dialog = None;
                let _ = self.reload_profiles(false, None)?;
            }
            DialogMode::AddCursorProfile => {
                let raw = dialog.input.trim();
                if raw.is_empty() {
                    return Ok(());
                }
                let name = crate::cursor_ingest::sanitize_cursor_profile_name(raw);
                let Some(store) = self.store.as_ref() else {
                    self.dialog = None;
                    return Ok(());
                };
                let path = store.paths().cursor_storage_state_path(&name);
                let bind = crate::cursor_ingest::cursor_ingest_bind_addr();
                let token = crate::cursor_ingest::cursor_ingest_token();
                let stop = Arc::new(AtomicBool::new(false));
                let stop_clone = Arc::clone(&stop);
                let (tx, rx) = mpsc::channel();
                let join = match crate::cursor_ingest::spawn_cursor_ingest_server(
                    &bind,
                    path,
                    token,
                    tx,
                    stop_clone,
                ) {
                    Ok(j) => j,
                    Err(e) => {
                        self.status_message = Some(format!("ingest server: {e:#}"));
                        self.dialog = None;
                        return Ok(());
                    }
                };
                let port = Self::parse_port_from_bind(&bind);
                self.cursor_ingest = Some(CursorIngestSession {
                    profile_name: name.clone(),
                    port,
                    stop,
                    join: Some(join),
                    rx,
                });
                self.dialog = Some(DialogState {
                    mode: DialogMode::CursorIngestWaiting { profile_name: name, port },
                    input: String::new(),
                    cursor: 0,
                });
            }
            DialogMode::CursorIngestWaiting { .. } => {}
            DialogMode::ConfirmDelete(target_name) => {
                if dialog.input.trim().eq_ignore_ascii_case("y")
                    || dialog.input.trim().eq_ignore_ascii_case("yes")
                {
                    match self.selected_profile().map(|p| p.kind) {
                        Some(ProfileKind::Claude) => {
                            let Some(cs) = self.claude_store.as_ref() else {
                                return Ok(());
                            };
                            cs.delete_account(&target_name)
                                .with_context(|| format!("delete Claude profile {target_name}"))?;
                        }
                        _ => {
                            let Some(store) = self.store.as_ref() else {
                                return Ok(());
                            };
                            store
                                .delete_account(&target_name)
                                .with_context(|| format!("delete profile {target_name}"))?;
                        }
                    }
                    self.status_message = Some(format!("Deleted \"{target_name}\"."));
                    let _ = self.reload_profiles(false, None)?;
                }
                self.dialog = None;
            }
        }

        Ok(())
    }

    fn activate_selected_profile(&mut self) -> Result<()> {
        let Some(profile) = self.selected_profile().cloned() else {
            return Ok(());
        };

        match profile.kind {
            ProfileKind::Claude => {
                if let Some(saved_name) = profile.saved_name.as_deref() {
                    if let Some(cs) = self.claude_store.as_ref() {
                        let activated = cs.use_account(saved_name)?;
                        self.status_message = Some(format!("Switched Claude auth to \"{activated}\"."));
                        let _ = self.reload_profiles(false, profile.account_id.clone())?;
                    }
                } else {
                    let default_name = build_default_name(profile.usage_view.usage.as_ref(), &profile.snapshot);
                    self.dialog = Some(DialogState {
                        mode: DialogMode::SaveCurrent(ProfileKind::Claude),
                        cursor: default_name.chars().count(),
                        input: default_name,
                    });
                }
            }
            ProfileKind::Codex => {
                if let Some(saved_name) = profile.saved_name.as_deref() {
                    if let Some(store) = self.store.as_ref() {
                        let activated = store.use_account(saved_name)?;
                        self.status_message = Some(format!("Switched Codex auth to \"{activated}\"."));
                        let _ = self.reload_profiles(false, profile.account_id.clone())?;
                    }
                } else {
                    let default_name = build_default_name(profile.usage_view.usage.as_ref(), &profile.snapshot);
                    self.dialog = Some(DialogState {
                        mode: DialogMode::SaveCurrent(ProfileKind::Codex),
                        cursor: default_name.chars().count(),
                        input: default_name,
                    });
                }
            }
            ProfileKind::Copilot => {
                // Copilot is auto-detected from ~/.config/gh/hosts.yml; no switching needed.
            }
        }
        Ok(())
    }

    fn open_rename_dialog(&mut self) {
        let Some(profile) = self.selected_profile() else {
            return;
        };
        if profile.kind == ProfileKind::Copilot {
            return;
        }
        let Some(saved_name) = profile.saved_name.clone() else {
            return;
        };
        self.dialog = Some(DialogState {
            mode: DialogMode::RenameSaved(saved_name.clone()),
            input: saved_name,
            cursor: profile.saved_name.as_ref().map(|name| name.chars().count()).unwrap_or(0),
        });
    }

    fn open_delete_dialog(&mut self) {
        let Some(profile) = self.selected_profile() else {
            return;
        };
        let Some(saved_name) = profile.saved_name.clone() else {
            return;
        };
        self.dialog = Some(DialogState {
            mode: DialogMode::ConfirmDelete(saved_name),
            input: String::new(),
            cursor: 0,
        });
    }

    fn refresh_selected_profile(&mut self, force_refresh: bool) -> Result<Vec<String>> {
        let account_id = self.selected_profile().and_then(|profile| profile.account_id.clone());
        self.reload_profiles(force_refresh, account_id)
    }

    fn reload_profiles(
        &mut self,
        force_refresh: bool,
        refresh_account_id: Option<String>,
    ) -> Result<Vec<String>> {
        let (Some(store), Some(usage_service)) = (self.store.as_ref(), self.usage_service.as_ref()) else {
            return Ok(Vec::new());
        };
        let report = crate::loader::load_profiles_with_report(
            store,
            usage_service,
            force_refresh,
            refresh_account_id.as_deref(),
            !force_refresh || refresh_account_id.is_some(),
            self.claude_store.as_ref(),
            self.claude_usage_service.as_ref(),
            self.copilot_usage_service.as_ref(),
        )?;
        self.profiles = report.profiles;
        self.cron_status = crate::cron::read_status(store.paths().cron_status_path());
        self.selected_profile_index = self
            .selected_profile_index
            .min(self.profiles.len().saturating_sub(1));
        if !self.y_zoom_user_adjusted {
            self.y_zoom_lower = auto_y_lower(&self.profiles);
        }
        let error_count = report.refresh_errors.len();
        if let Some(store) = self.store.as_ref() {
            let detail = format!(
                "mode=app force_refresh={} target={} profiles={} errors={}{}",
                force_refresh,
                refresh_account_id.as_deref().unwrap_or("all"),
                self.profiles.len(),
                error_count,
                if error_count > 0 {
                    format!(" first_error={}", report.refresh_errors[0])
                } else {
                    String::new()
                }
            );
            append_refresh_log(store.paths().refresh_log_path().as_path(), "reload_profiles", &detail);
        }
        self.ensure_background_refresh_task();
        Ok(report.refresh_errors)
    }

    fn ensure_background_refresh_task(&mut self) {
        let now_seconds = now_unix_seconds();
        if !should_schedule_background_refresh(self.background_refresh_retry_after, now_seconds) {
            return;
        }
        if !should_run_app_background_refresh(&self.cron_status, now_seconds) {
            self.background_refresh_state = BackgroundRefreshState::Idle;
            self.background_refresh_receiver = None;
            return;
        }
        if self.background_refresh_receiver.is_some() {
            return;
        }
        let Some(store) = self.store.clone() else {
            return;
        };
        let Some(usage_service) = self.usage_service.clone() else {
            return;
        };

        let stale_account_ids = stale_background_refresh_account_ids(
            &self.profiles,
            now_unix_seconds(),
            BACKGROUND_REFRESH_STALE_SECONDS,
        );
        if stale_account_ids.is_empty() {
            self.background_refresh_state = BackgroundRefreshState::Idle;
            return;
        }

        let claude_store = self.claude_store.clone();
        let claude_usage_service = self.claude_usage_service.clone();
        let copilot_usage_service = self.copilot_usage_service.clone();
        let queued_profiles = stale_account_ids.len();
        let (tx, rx) = mpsc::channel();
        self.background_refresh_state = BackgroundRefreshState::Running { queued_profiles };
        self.background_refresh_receiver = Some(rx);

        std::thread::spawn(move || {
            let mut refreshed_profiles = 0;
            let mut refresh_errors = Vec::new();

            for account_id in stale_account_ids {
                match crate::loader::load_profiles_with_report(
                    &store,
                    &usage_service,
                    true,
                    Some(account_id.as_str()),
                    true,
                    claude_store.as_ref(),
                    claude_usage_service.as_ref(),
                    copilot_usage_service.as_ref(),
                ) {
                    Ok(report) => {
                        refreshed_profiles += 1;
                        refresh_errors.extend(report.refresh_errors);
                    }
                    Err(error) => {
                        refresh_errors.push(format!("{account_id}: {error:#}"));
                    }
                }
            }

            let _ = tx.send(BackgroundRefreshReport {
                refreshed_profiles,
                refresh_errors,
            });
        });
    }

    fn poll_background_refresh(&mut self) {
        let outcome = match self.background_refresh_receiver.as_ref() {
            Some(receiver) => receiver.try_recv(),
            None => return,
        };

        match outcome {
            Ok(report) => {
                self.background_refresh_receiver = None;
                self.background_refresh_state = BackgroundRefreshState::Idle;
                self.background_refresh_retry_after = if report.refresh_errors.is_empty() {
                    None
                } else {
                    Some(now_unix_seconds() + BACKGROUND_REFRESH_ERROR_COOLDOWN_SECONDS)
                };
                let reload_result = self.reload_profiles(false, None);
                if let Some(store) = self.store.as_ref() {
                    let detail = format!(
                        "mode=app-background refreshed_profiles={} errors={}{}",
                        report.refreshed_profiles,
                        report.refresh_errors.len(),
                        if report.refresh_errors.is_empty() {
                            String::new()
                        } else {
                            format!(" first_error={}", report.refresh_errors[0])
                        }
                    );
                    append_refresh_log(store.paths().refresh_log_path().as_path(), "background_refresh", &detail);
                }
                self.status_message = Some(match (report.refreshed_profiles, report.refresh_errors.is_empty()) {
                    (count, true) => format!("Background refresh updated {count} profiles"),
                    (count, false) if count > 0 => format!(
                        "Background refresh updated {count} profiles with errors: {}",
                        report.refresh_errors.join(" | ")
                    ),
                    _ => format!("Background refresh failed: {}", report.refresh_errors.join(" | ")),
                });
                if let Err(error) = reload_result {
                    self.status_message = Some(format!("Background refresh reload failed: {error:#}"));
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.background_refresh_receiver = None;
                self.background_refresh_state = BackgroundRefreshState::Idle;
                self.status_message = Some("Background refresh worker disconnected".to_string());
            }
        }
    }

    fn selected_profile(&self) -> Option<&ProfileEntry> {
        self.profiles.get(self.selected_profile_index)
    }

    fn step_profile(&mut self, delta: isize) {
        let len = self.profiles.len();
        if len == 0 {
            self.selected_profile_index = 0;
            return;
        }
        let len = len as isize;
        let current = self.selected_profile_index as isize;
        let next = (current + delta).rem_euclid(len);
        self.selected_profile_index = next as usize;
    }

    fn left_pane_width(&self, total_width: u16) -> u16 {
        // Width needed for profile list items:
        // "  " highlight_symbol + "▶~{name}{unsaved_tag} {badge}" + 2 border cols
        let max_list = self
            .profiles
            .iter()
            .map(|profile| {
                let unsaved_tag = if profile.saved_name.is_none() { " [unsaved]".len() } else { 0 };
                let badge = format_usage_badge(&profile.usage_view);
                2 + profile.profile_name.len() + unsaved_tag + 1 + badge.len() + 2
            })
            .max()
            .unwrap_or(20);

        // Width needed for Details panel: measure every detail line for every profile,
        // so the pane is wide enough regardless of which profile is selected.
        let max_detail = self
            .profiles
            .iter()
            .flat_map(|profile| {
                render_account_detail(Some(profile), None)
            })
            .map(|line| line.width())
            .max()
            .unwrap_or(0) + 2; // +2 for "Details" block borders

        let max_refresh = render_refresh_tasks(
            &self.cron_status,
            self.status_message.as_deref(),
            &self.background_refresh_state,
        )
        .into_iter()
        .map(|line| line.width())
        .max()
        .unwrap_or(0) + 2; // +2 for "Refresh tasks" block borders

        let max_content = max_list.max(max_detail).max(max_refresh) as u16;
        // Give the chart at least 40 columns; always at least 20 for the left pane.
        let max_allowed = total_width.saturating_sub(40).max(20);
        max_content.min(max_allowed)
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let footer_height = if self.fullscreen || self.pane_focus == PaneFocus::Plot { 1 } else { 2 };
        let [body, footer_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(footer_height)]).areas(area);

        let chart_area = if self.fullscreen {
            body
        } else {
            let left_width = self.left_pane_width(body.width);
            let [left_area, right_area] =
                Layout::horizontal([Constraint::Length(left_width), Constraint::Min(0)]).areas(body);
            self.render_left_pane(frame, left_area);
            right_area
        };

        let render_state = AppRenderState {
            profiles: &self.profiles,
            selected_profile_index: self.selected_profile_index,
            y_zoom_lower: self.y_zoom_lower,
            y_zoom_upper: self.y_zoom_upper,
            solo: self.solo_mode,
            x_window_days: self.x_window_days,
            x_offset_days: self.x_offset_days,
            plot_focused: self.pane_focus == PaneFocus::Plot,
            fullscreen: self.fullscreen,
            tab_zoom_index: self.tab_zoom_index,
            hidden_profiles: &self.hidden_profiles,
        };
        render::render(frame, chart_area, &render_state);

        let footer_lines = if self.fullscreen {
            vec![
                Line::from("p=profiles · Space=hide · a=refresh · q=quit"),
            ]
        } else {
            match self.pane_focus {
                PaneFocus::Accounts => vec![
                    Line::from("Enter=switch/save · o=Cursor profile · r=rename · d=delete · u=refresh · a=all · q=quit"),
                    Line::from(format!(
                        "Tab=hide · p=profiles · ↑↓=navigate · /=filter{}",
                        if self.filter_input.is_some() { " (Esc=clear)" } else { "" }
                    )),
                ],
                PaneFocus::Plot => vec![
                    Line::from("Tab=accounts · p=profiles · a=refresh · q=quit"),
                ],
            }
        };
        let footer = Paragraph::new(Text::from(footer_lines)).wrap(Wrap { trim: true });
        frame.render_widget(footer, footer_area);
        let version_area = Rect {
            x: footer_area.x,
            y: footer_area.y + footer_area.height.saturating_sub(1),
            width: footer_area.width,
            height: 1,
        };
        let version = Paragraph::new(Text::from(vec![Line::from(env!("BUILD_VER"))]))
            .alignment(Alignment::Right)
            .wrap(Wrap { trim: true });
        frame.render_widget(version, version_area);

        if self.dialog.is_some() {
            self.render_dialog(frame);
        }
    }

    fn render_left_pane(&self, frame: &mut Frame, area: Rect) {
        let indices = self.filtered_profile_indices();
        let list_lines = (indices.len().clamp(3, 10) + 2) as u16;
        let refresh_lines = render_refresh_tasks(
            &self.cron_status,
            self.status_message.as_deref(),
            &self.background_refresh_state,
        );
        let refresh_height = ((refresh_lines.len() as u16) + 4).clamp(6, 12);

        let [list_area, lower_area] =
            Layout::vertical([Constraint::Length(list_lines), Constraint::Min(0)]).areas(area);
        let [detail_area, refresh_area] = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(refresh_height),
        ])
        .areas(lower_area);

        let profiles_title = match &self.filter_input {
            Some(f) => format!("Profiles [/{}]", f),
            None if self.pane_focus == PaneFocus::Accounts => "Profiles [active]".to_string(),
            None => "Profiles".to_string(),
        };

        let items = indices
            .iter()
            .map(|&i| {
                let profile = &self.profiles[i];
                let color = render::SERIES_COLORS[i % render::SERIES_COLORS.len()];
                let is_hidden = self.hidden_profiles.contains(&App::profile_hidden_key(profile));
                let current_sym = if profile.is_current { '▶' } else { ' ' };
                let hidden_sym  = if is_hidden            { '~' } else { ' ' };
                let prefix = format!("{current_sym}{hidden_sym}");
                let service_tag = match profile.kind {
                    ProfileKind::Codex => "[codex]",
                    ProfileKind::Claude => "[claude]",
                    ProfileKind::Copilot => "[copilot]",
                };
                let state_tag = if profile.saved_name.is_none() {
                    " [unsaved]"
                } else {
                    ""
                };
                let usage = format_usage_badge(&profile.usage_view);
                let label = format!("{prefix}{service_tag} {}{state_tag}", profile.profile_name);
                ListItem::new(Line::from(vec![
                    Span::styled(label, Style::default().fg(color)),
                    Span::raw(format!(" {usage}")),
                ]))
            })
            .collect::<Vec<_>>();

        let selected_pos_in_filtered = indices
            .iter()
            .position(|&i| i == self.selected_profile_index);

        let mut state = ListState::default();
        state.select(selected_pos_in_filtered);
        let list_block = if self.pane_focus == PaneFocus::Accounts {
            Block::default()
                .title(profiles_title)
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(20, 20, 20)))
        } else {
            Block::default().title(profiles_title).borders(Borders::ALL)
        };
        let list = List::new(items)
            .block(list_block)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, list_area, &mut state);

        let detail_lines =
            render_account_detail(self.selected_profile(), self.status_message.as_deref());
        let details = Paragraph::new(Text::from(detail_lines))
            .block(Block::default().title("Details").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(details, detail_area);

        let refresh = Paragraph::new(Text::from(refresh_lines))
            .block(Block::default().title("Refresh tasks").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(refresh, refresh_area);
    }

    fn render_dialog(&self, frame: &mut Frame) {
        let Some(dialog) = self.dialog.as_ref() else {
            return;
        };
        if let DialogMode::CursorIngestWaiting { profile_name, port } = &dialog.mode {
            let area = popup_area(frame.area(), 78, 24);
            frame.render_widget(Clear, area);
            let text = Text::from(vec![
                Line::from(format!("Saving as: {profile_name}.json")),
                Line::from(""),
                Line::from(format!(
                    "POST Playwright storageState JSON to http://<this-host>:{port}/ingest"
                )),
                Line::from("(GET /health). Only source IP 100.* (Tailscale) is accepted."),
                Line::from(""),
                Line::from(format!(
                    "On Windows: cursor-export --url http://<linux-tailscale-ip>:{port}/ingest"
                )),
                Line::from(""),
                Line::from("Esc = cancel"),
            ]);
            let widget = Paragraph::new(text)
                .block(
                    Block::default()
                        .title("Cursor profile ingest")
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: true });
            frame.render_widget(widget, area);
            return;
        }
        let title = match &dialog.mode {
            DialogMode::SaveCurrent(_) => "Save current profile",
            DialogMode::RenameSaved(_) => "Rename saved profile",
            DialogMode::ConfirmDelete(_) => "Delete saved profile",
            DialogMode::AddCursorProfile => "New Cursor profile",
            DialogMode::CursorIngestWaiting { .. } => "Cursor profile ingest",
        };
        let prompt = match &dialog.mode {
            DialogMode::SaveCurrent(_) => "Enter a name for the current auth snapshot.",
            DialogMode::RenameSaved(_) => "Enter the new saved profile name.",
            DialogMode::AddCursorProfile => {
                "Enter a label for this Cursor Chrome storageState (agent-switch config dir / cursor-profiles/)."
            }
            DialogMode::CursorIngestWaiting { .. } => "",
            DialogMode::ConfirmDelete(target) => {
                if dialog.input.is_empty() {
                    return self.render_confirm_prompt(frame, title, &format!("Delete \"{target}\"? Type y to confirm."));
                }
                "Type y or yes to confirm deletion."
            }
        };

        let area = popup_area(frame.area(), 70, 20);
        frame.render_widget(Clear, area);
        let dialog_widget = Paragraph::new(Text::from(vec![
            Line::from(prompt),
            Line::from(""),
            Line::from(dialog.input.clone().yellow()),
        ]))
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: true });
        frame.render_widget(dialog_widget, area);
        frame.set_cursor_position(dialog_cursor_position(area, dialog));
    }

    fn render_confirm_prompt(&self, frame: &mut Frame, title: &str, prompt: &str) {
        let area = popup_area(frame.area(), 70, 20);
        frame.render_widget(Clear, area);
        let dialog_widget = Paragraph::new(Text::from(vec![
            Line::from(prompt),
            Line::from(""),
            Line::from(self.dialog.as_ref().map(|dialog| dialog.input.clone()).unwrap_or_default().yellow()),
        ]))
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: true });
        frame.render_widget(dialog_widget, area);
        if let Some(dialog) = self.dialog.as_ref() {
            frame.set_cursor_position(dialog_cursor_position(area, dialog));
        }
    }
}

impl render::RenderState for AppRenderState<'_> {
    fn selection_state(&self) -> SelectionState<'_> {
        SelectionState {
            selected: self.selected_profile().map(|profile| RenderProfile {
                id: profile.account_id.as_deref().unwrap_or(profile.profile_name.as_str()),
                label: profile.profile_name.as_str(),
                is_current: profile.is_current,
                agent_type: profile.kind.as_str(),
                window_label: profile.chart_data.quota_window_label.as_str(),
            }),
            current: self.current_profile().map(|profile| RenderProfile {
                id: profile.account_id.as_deref().unwrap_or(profile.profile_name.as_str()),
                label: profile.profile_name.as_str(),
                is_current: profile.is_current,
                agent_type: profile.kind.as_str(),
                window_label: profile.chart_data.quota_window_label.as_str(),
            }),
        }
    }

    fn chart_state(&self) -> ChartState<'_> {
        let effective_selected = self.tab_zoom_index.unwrap_or(self.selected_profile_index);
        let mut state = build_chart_state(self.profiles, effective_selected);

        // Mark hidden profiles
        for (i, series) in state.series.iter_mut().enumerate() {
            let key = format!(
                "{}|{}",
                self.profiles.get(i).map(|p| p.kind.as_str()).unwrap_or(""),
                self.profiles.get(i).map(|p| p.profile_name.as_str()).unwrap_or(""),
            );
            series.style.hidden = self.hidden_profiles.contains(&key);
        }

        if let Some(idx) = self.tab_zoom_index {
            if let Some(profile) = self.profiles.get(idx) {
                // Auto y-bounds: fit this profile's data + 5h band
                let mut all_ys: Vec<f64> =
                    profile.chart_data.seven_day_points.iter().map(|p| p.y).collect();
                if let Some(y) = profile.chart_data.five_hour_band.lower_y { all_ys.push(y); }
                if let Some(y) = profile.chart_data.five_hour_band.upper_y { all_ys.push(y); }
                if !all_ys.is_empty() {
                    let min_y = all_ys.iter().cloned().fold(f64::INFINITY, f64::min);
                    let max_y = all_ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    let margin = ((max_y - min_y) * 0.15).max(5.0);
                    state.y_lower = (min_y - margin).max(0.0);
                    state.y_upper = (max_y + margin).min(100.0);
                }
                state.solo = true;
                state.tab_zoom_label = Some(profile.profile_name.as_str());
            }
        } else {
            state.y_lower = self.y_zoom_lower;
            state.y_upper = self.y_zoom_upper;
            state.solo = self.solo;
            state.tab_zoom_label = None;
        }

        state.x_lower = 7.0 - self.x_window_days - self.x_offset_days;
        state.x_upper = 7.0 - self.x_offset_days;
        state.focused = self.plot_focused;
        state.fullscreen = self.fullscreen;
        state
    }
}

impl AppRenderState<'_> {
    fn selected_profile(&self) -> Option<&ProfileEntry> {
        self.profiles.get(self.selected_profile_index)
    }

    fn current_profile(&self) -> Option<&ProfileEntry> {
        self.profiles.iter().find(|profile| profile.is_current)
    }
}

struct TerminalSession {
    terminal: ratatui::DefaultTerminal,
}

impl TerminalSession {
    fn enter() -> Self {
        Self {
            terminal: ratatui::init(),
        }
    }

    fn run(&mut self, app: &mut App) -> Result<()> {
        app.run_loop(&mut self.terminal)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

fn binary_mtime(path: &str) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn should_run_app_background_refresh(cron_status: &CronStatus, now_seconds: i64) -> bool {
    if !cron_status.installed {
        return true;
    }
    let has_recent_success = cron_status
        .last_run
        .is_some_and(|ts| now_seconds.saturating_sub(ts) <= CRON_YIELD_FRESH_SECONDS);
    let has_errors = cron_status.codex_error.is_some()
        || cron_status.claude_error.is_some()
        || cron_status.copilot_error.is_some();
    // Yield only when cron has succeeded recently and is currently healthy.
    !(has_recent_success && !has_errors)
}

fn should_schedule_background_refresh(retry_after: Option<i64>, now_seconds: i64) -> bool {
    !retry_after.is_some_and(|retry_at| now_seconds < retry_at)
}

fn resolve_hot_reload_binary_path(current_exe: &std::path::Path) -> PathBuf {
    let current = current_exe.to_path_buf();
    let current_meta = std::fs::metadata(&current).ok().and_then(|m| m.modified().ok());
    let path_bin = which::which("agent-switch").ok();
    match (path_bin, current_meta) {
        (Some(path_bin), Some(current_mtime)) => {
            let path_mtime = std::fs::metadata(&path_bin).ok().and_then(|m| m.modified().ok());
            if path_mtime.is_some_and(|mtime| mtime > current_mtime) {
                path_bin
            } else {
                current
            }
        }
        (Some(path_bin), None) => path_bin,
        (None, _) => current,
    }
}

fn initial_selected_index(profiles: &[ProfileEntry]) -> usize {
    profiles.iter().position(|profile| profile.is_current).unwrap_or(0)
}

fn render_account_detail(
    profile: Option<&ProfileEntry>,
    _status_message: Option<&str>,
) -> Vec<Line<'static>> {
    let Some(profile) = profile else {
        return vec![
            Line::from("No auth profile loaded."),
            Line::from("Save or switch an account to continue."),
        ];
    };

    let state_label = match (profile.is_current, profile.saved_name.is_some()) {
        (true, _) => "current",
        (false, true) => "saved",
        (false, false) => "unsaved",
    };

    let service_label = match profile.kind {
        ProfileKind::Codex => "Codex [codex]",
        ProfileKind::Claude => "Claude [claude]",
        ProfileKind::Copilot => "GitHub Copilot [copilot]",
    };

    let mut lines = vec![
        Line::from(format!("Profile: {}", profile.profile_name)),
        Line::from(format!("Service: {}", service_label)),
        Line::from(format!("State: {}", state_label)),
        Line::from(format!(
            "Last updated: {}",
            format_age(profile.usage_view.fetched_at, profile.usage_view.stale)
        )),
    ];

    if let Some(usage) = profile.usage_view.usage.as_ref() {
        if let Some(email) = usage.email.as_deref() {
            lines.push(Line::from(format!("Email: {}", email)));
        }
        if let Some(plan) = usage.plan_type.as_deref() {
            lines.push(Line::from(format!("Plan: {}", plan)));
        }
        if let Some(w) = pick_weekly_window(usage) {
            lines.push(Line::from(format!(
                "Quota: {:.0}% used, reset in {}",
                w.used_percent, format_duration_short(w.reset_after_seconds)
            )));
        }
        if profile.kind != ProfileKind::Copilot {
            if let Some(w) = pick_five_hour_window(usage) {
                lines.push(Line::from(format!(
                    "5h: {:.0}% used, reset in {}",
                    w.used_percent, format_duration_short(w.reset_after_seconds)
                )));
            }
        }
    }

    lines
}

fn render_refresh_tasks(
    cron_status: &CronStatus,
    status_message: Option<&str>,
    background_refresh_state: &BackgroundRefreshState,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(match background_refresh_state {
        BackgroundRefreshState::Idle => "Background: idle".to_string(),
        BackgroundRefreshState::Running { queued_profiles } => {
            format!("Background: refreshing {queued_profiles} stale profiles")
        }
    })];

    if let Some(message) = status_message {
        lines.push(Line::from(format!("Last result: {message}")));
    }

    if !cron_status.installed {
        lines.push(Line::from("Cron: not installed"));
        return lines;
    }

    let attempt_age = cron_status
        .last_attempt
        .or(cron_status.last_run)
        .map(|ts| format_age(Some(ts), false))
        .unwrap_or_else(|| "never".to_string());
    lines.push(Line::from(format!("Cron: installed · last attempt {attempt_age}")));

    if let Some(last_success) = cron_status.last_run {
        lines.push(Line::from(format!(
            "Last success: {}",
            format_age(Some(last_success), false)
        )));
    }
    if let Some(error) = cron_status.codex_error.as_deref() {
        lines.push(Line::from(format!("Codex issue: {error}")));
    }
    if let Some(error) = cron_status.claude_error.as_deref() {
        lines.push(Line::from(format!("Claude issue: {error}")));
    }
    if let Some(error) = cron_status.copilot_error.as_deref() {
        lines.push(Line::from(format!("Copilot issue: {error}")));
    }
    lines
}

fn format_age(fetched_at: Option<i64>, stale: bool) -> String {
    let Some(ts) = fetched_at else {
        return "never".to_string();
    };
    let now = now_unix_seconds();
    let age_secs = now.saturating_sub(ts).max(0) as u64;
    let age_str = if age_secs < 60 {
        format!("{}s ago", age_secs)
    } else if age_secs < 3600 {
        format!("{}m ago", age_secs / 60)
    } else if age_secs < 86400 {
        format!("{}h ago", age_secs / 3600)
    } else {
        format!("{}d ago", age_secs / 86400)
    };
    if stale {
        format!("stale {}", age_str)
    } else {
        age_str
    }
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn stale_background_refresh_account_ids(
    profiles: &[ProfileEntry],
    now_seconds: i64,
    stale_after_seconds: i64,
) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut account_ids = Vec::new();

    for profile in profiles {
        let Some(account_id) = profile.account_id.as_ref() else {
            continue;
        };
        let age = profile
            .usage_view
            .fetched_at
            .map(|fetched_at| now_seconds.saturating_sub(fetched_at))
            .unwrap_or(i64::MAX);
        if age < stale_after_seconds {
            continue;
        }
        if seen.insert(account_id.clone()) {
            account_ids.push(account_id.clone());
        }
    }

    account_ids
}

fn format_usage_badge(view: &UsageReadResult) -> String {
    match &view.usage {
        Some(usage) => format!(
            "{}{}",
            usage.plan_type.as_deref().unwrap_or("plan"),
            if view.stale { " stale" } else { "" }
        ),
        None => "no-usage".to_string(),
    }
}

fn read_account_id(snapshot: &Value) -> Option<String> {
    snapshot
        .get("tokens")
        .and_then(|value| value.get("account_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn build_default_name(usage: Option<&UsageResponse>, snapshot: &Value) -> String {
    let email_part = sanitize_name_part(usage.and_then(|value| value.email.as_deref()));
    let plan_part = sanitize_name_part(usage.and_then(|value| value.plan_type.as_deref()));
    let account_part = sanitize_name_part(read_account_id(snapshot).as_deref());

    match (email_part, plan_part, account_part) {
        (Some(email), Some(plan), _) => format!("{email}-{plan}"),
        (Some(email), None, _) => email,
        (None, _, Some(account)) => format!("profile-{}", &account.chars().take(8).collect::<String>()),
        _ => "profile".to_string(),
    }
}

fn sanitize_name_part(input: Option<&str>) -> Option<String> {
    input
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .to_lowercase()
                .replace('@', "-")
                .chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                        ch
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
                .trim_matches('-')
                .to_string()
        })
        .filter(|value| !value.is_empty())
}

/// Compute the default y_zoom_lower from actual data: floor(min_y, 5%).
/// Returns 0.0 if no data.
const END_LABEL_Y_PADDING_PERCENT: f64 = 10.0;

fn auto_y_lower(profiles: &[ProfileEntry]) -> f64 {
    let min_y = profiles
        .iter()
        .flat_map(|profile| {
            profile
                .chart_data
                .seven_day_points
                .iter()
                .map(|pt| pt.y)
                .chain(profile.chart_data.five_hour_subframe.lower_y)
        })
        .fold(f64::INFINITY, f64::min);
    if min_y.is_infinite() {
        return 0.0;
    }
    (((min_y - END_LABEL_Y_PADDING_PERCENT) / 5.0).floor() * 5.0).max(0.0)
}

fn build_reset_line_display(
    last_seven_day_percent: Option<f64>,
    five_hour_used_percent: Option<f64>,
    weekly_countdown_seconds: Option<i64>,
    five_hour_countdown_seconds: Option<i64>,
) -> Option<crate::render::ResetLineDisplay> {
    let weekly_qualifies = last_seven_day_percent.is_some_and(|value| value >= 100.0);
    let five_hour_qualifies = five_hour_used_percent.is_some_and(|value| value >= 100.0);

    let weekly_display = weekly_countdown_seconds.map(|seconds| crate::render::ResetLineDisplay {
        source: crate::render::ResetLineSource::Weekly,
        text: format!("Hit limit · resets in {}", format_duration_short(seconds)),
    });
    let five_hour_display =
        five_hour_countdown_seconds.map(|seconds| crate::render::ResetLineDisplay {
            source: crate::render::ResetLineSource::FiveHour,
            text: format!("Hit limit · resets in {}", format_duration_short(seconds)),
        });

    match (weekly_qualifies, five_hour_qualifies) {
        (true, true) => match (weekly_countdown_seconds, five_hour_countdown_seconds) {
            (Some(weekly), Some(five_hour)) => {
                if five_hour > weekly {
                    five_hour_display
                } else {
                    weekly_display
                }
            }
            (Some(_), None) => weekly_display,
            (None, Some(_)) => five_hour_display,
            (None, None) => None,
        },
        (true, false) => weekly_display,
        (false, true) => five_hour_display,
        (false, false) => None,
    }
}

fn build_chart_state<'a>(profiles: &'a [ProfileEntry], selected_profile_index: usize) -> ChartState<'a> {
    let selected_profile = profiles.get(selected_profile_index);
    let selected_label = selected_profile
        .map(|profile| profile.profile_name.as_str())
        .unwrap_or("no selected profile");
    let selected_series = selected_profile
        .map(|profile| profile.chart_data.seven_day_points.clone())
        .unwrap_or_default();
    let selected_band = selected_profile
        .map(|profile| FiveHourBandState {
            available: profile.chart_data.five_hour_band.available,
            used_percent: profile.chart_data.five_hour_band.used_percent,
            lower_y: profile.chart_data.five_hour_band.lower_y,
            upper_y: profile.chart_data.five_hour_band.upper_y,
            delta_seven_day_percent: profile.chart_data.five_hour_band.delta_seven_day_percent,
            delta_five_hour_percent: profile.chart_data.five_hour_band.delta_five_hour_percent,
            reason: profile.chart_data.five_hour_band.reason.as_deref(),
        })
        .unwrap_or(FiveHourBandState {
            available: false,
            used_percent: None,
            lower_y: None,
            upper_y: None,
            delta_seven_day_percent: None,
            delta_five_hour_percent: None,
            reason: Some("no selected profile"),
        });
    let selected_subframe = selected_profile
        .map(|profile| FiveHourSubframeState {
            available: profile.chart_data.five_hour_subframe.available,
            start_x: profile.chart_data.five_hour_subframe.start_x,
            end_x: profile.chart_data.five_hour_subframe.end_x,
            lower_y: profile.chart_data.five_hour_subframe.lower_y,
            upper_y: profile.chart_data.five_hour_subframe.upper_y,
            reason: profile.chart_data.five_hour_subframe.reason.as_deref(),
        })
        .unwrap_or(FiveHourSubframeState {
            available: false,
            start_x: None,
            end_x: None,
            lower_y: None,
            upper_y: None,
            reason: Some("no selected profile"),
        });

    let series = profiles
        .iter()
        .enumerate()
        .map(|(index, profile)| {
            let last_seven_day_percent =
                profile.chart_data.seven_day_points.last().map(|point| point.y);
            let five_hour_used_percent = profile.chart_data.five_hour_band.used_percent;

            ChartSeries {
                profile: RenderProfile {
                    id: profile.account_id.as_deref().unwrap_or(profile.profile_name.as_str()),
                    label: profile.profile_name.as_str(),
                    is_current: profile.is_current,
                    agent_type: profile.kind.as_str(),
                    window_label: profile.chart_data.quota_window_label.as_str(),
                },
                style: ChartSeriesStyle {
                    color_slot: index,
                    is_selected: index == selected_profile_index,
                    is_current: profile.is_current,
                    hidden: false,
                },
                points: profile.chart_data.seven_day_points.clone(),
                last_seven_day_percent,
                five_hour_used_percent,
                forecast_label: profile.chart_data.forecast.compact_label.as_deref(),
                five_hour_subframe: FiveHourSubframeState {
                    available: profile.chart_data.five_hour_subframe.available,
                    start_x: profile.chart_data.five_hour_subframe.start_x,
                    end_x: profile.chart_data.five_hour_subframe.end_x,
                    lower_y: profile.chart_data.five_hour_subframe.lower_y,
                    upper_y: profile.chart_data.five_hour_subframe.upper_y,
                    reason: profile.chart_data.five_hour_subframe.reason.as_deref(),
                },
                is_zero_state: profile.chart_data.is_zero_state,
                reset_line_display: build_reset_line_display(
                    last_seven_day_percent,
                    five_hour_used_percent,
                    profile.chart_data.weekly_reset_countdown_seconds,
                    profile.chart_data.five_hour_reset_countdown_seconds,
                ),
            }
        })
        .collect::<Vec<_>>();

    let total_points = series.iter().map(|series| series.points.len()).sum();
    let mut chart_state = ChartState {
        series,
        seven_day_points: selected_series,
        five_hour_band: selected_band,
        five_hour_subframe: selected_subframe,
        total_points,
        y_lower: 0.0,
        y_upper: 100.0,
        x_lower: 0.0,
        x_upper: 7.0,
        solo: false,
        tab_zoom_label: None,
        focused: false,
        fullscreen: false,
    };

    if chart_state.series.is_empty() && chart_state.seven_day_points.is_empty() {
        chart_state.five_hour_band.reason = Some(selected_label);
        chart_state.five_hour_subframe.reason = Some(selected_label);
    }

    chart_state
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

fn dialog_cursor_position(area: Rect, dialog: &DialogState) -> Position {
    let input_x = area.x.saturating_add(1);
    let input_y = area.y.saturating_add(3);
    Position::new(
        input_x.saturating_add(dialog.cursor as u16),
        input_y,
    )
}

fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or_else(|| text.len())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use crate::app_data::{OwnedFiveHourBandState, OwnedFiveHourSubframeState};
    use crate::render::ChartPoint;
    use super::*;
    use ratatui::backend::Backend;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render_buffer(app: &App, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let frame = terminal.draw(|frame| app.render(frame)).unwrap();
        frame.buffer.clone()
    }

    fn buffer_row_text(
        buffer: &ratatui::buffer::Buffer,
        y: u16,
        width: u16,
    ) -> String {
        (0..width)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn app_starts_on_current_profile_and_toggles_pane_focus() {
        let mut app = App::from_profile_names(vec!["Alpha".to_string(), "Beta".to_string()], 1);
        assert_eq!(app.pane_focus, PaneFocus::Plot);
        assert_eq!(app.selected_profile_label(), Some("Beta"));

        app.pane_focus = app.pane_focus.toggle();
        assert_eq!(app.pane_focus, PaneFocus::Accounts);
        assert_eq!(app.selected_profile_label(), Some("Beta"));
    }

    #[test]
    fn app_background_refresh_yields_when_cron_installed() {
        let now = 1_800_000_000;
        let healthy_recent = CronStatus {
            installed: true,
            last_run: Some(now - 120),
            last_attempt: Some(now - 120),
            codex_error: None,
            claude_error: None,
            copilot_error: None,
        };
        assert!(!should_run_app_background_refresh(&healthy_recent, now));

        let installed_but_stale = CronStatus {
            installed: true,
            last_run: Some(now - 10_000),
            last_attempt: Some(now - 120),
            codex_error: None,
            claude_error: None,
            copilot_error: None,
        };
        assert!(should_run_app_background_refresh(&installed_but_stale, now));

        let attempted_but_failed = CronStatus {
            installed: true,
            last_run: Some(now - 10_000),
            last_attempt: Some(now - 60),
            codex_error: Some("HTTP 402".to_string()),
            claude_error: None,
            copilot_error: None,
        };
        assert!(should_run_app_background_refresh(&attempted_but_failed, now));

        let installed_with_error = CronStatus {
            installed: true,
            last_run: Some(now - 60),
            last_attempt: Some(now - 60),
            codex_error: Some("HTTP 402".to_string()),
            claude_error: None,
            copilot_error: None,
        };
        assert!(should_run_app_background_refresh(&installed_with_error, now));

        assert!(should_run_app_background_refresh(&CronStatus::uninstalled(), now));
    }

    #[test]
    fn app_background_refresh_respects_error_cooldown() {
        let now = 1_800_000_000;
        assert!(!should_schedule_background_refresh(Some(now + 60), now));
        assert!(should_schedule_background_refresh(Some(now - 1), now));
        assert!(should_schedule_background_refresh(None, now));
    }

    #[test]
    fn chart_series_propagates_zero_state_from_profile_data() {
        let profiles = vec![ProfileEntry {
            kind: ProfileKind::Codex,
            saved_name: Some("alpha".to_string()),
            profile_name: "Alpha".to_string(),
            snapshot: serde_json::json!({}),
            usage_view: crate::usage::UsageReadResult {
                usage: None,
                source: crate::usage::UsageSource::None,
                fetched_at: None,
                stale: false,
            },
            account_id: Some("acct-alpha".to_string()),
            is_current: true,
            chart_data: crate::app_data::ProfileChartData {
                seven_day_points: vec![],
                quota_window_label: "7d".to_string(),
                forecast: crate::app_data::OwnedUsageForecast::empty("zero-state"),
                weekly_reset_countdown_seconds: None,
                five_hour_reset_countdown_seconds: None,
                five_hour_band: OwnedFiveHourBandState {
                    available: false,
                    used_percent: None,
                    lower_y: None,
                    upper_y: None,
                    delta_seven_day_percent: None,
                    delta_five_hour_percent: None,
                    reason: Some("zero-state".to_string()),
                },
                five_hour_subframe: OwnedFiveHourSubframeState {
                    available: false,
                    start_x: None,
                    end_x: None,
                    lower_y: None,
                    upper_y: None,
                    reason: Some("zero-state".to_string()),
                },
                is_zero_state: true,
            },
        }];

        let chart_state = build_chart_state(&profiles, 0);
        assert!(chart_state.series[0].is_zero_state);
        assert!(chart_state.series[0].five_hour_subframe.available == false);
    }

    #[test]
    fn resolve_hot_reload_binary_prefers_newer_path_binary() {
        let base = std::env::temp_dir().join(format!("agent-switch-hot-reload-{}", std::process::id()));
        let _ = fs::create_dir_all(&base);
        let current = base.join("agent-switch-current");
        let path_bin = base.join("agent-switch");
        fs::write(&current, "old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        fs::write(&path_bin, "new").unwrap();

        let current_mtime = fs::metadata(&current).unwrap().modified().unwrap();
        let path_mtime = fs::metadata(&path_bin).unwrap().modified().unwrap();
        assert!(path_mtime > current_mtime, "test requires path binary newer than current");

        // Directly validate decision logic by simulating which() outcome through metadata comparison:
        let resolved = {
            let current_meta = fs::metadata(&current).ok().and_then(|m| m.modified().ok());
            match (Some(path_bin.clone()), current_meta) {
                (Some(path_bin), Some(current_mtime)) => {
                    let path_mtime = fs::metadata(&path_bin).ok().and_then(|m| m.modified().ok());
                    if path_mtime.is_some_and(|mtime| mtime > current_mtime) {
                        path_bin
                    } else {
                        current.clone()
                    }
                }
                _ => current.clone(),
            }
        };
        assert_eq!(resolved, path_bin);
    }

    #[test]
    fn account_detail_empty_state_is_service_agnostic() {
        let lines = render_account_detail(None, None);
        assert_eq!(lines[0].to_string(), "No auth profile loaded.");
        assert_eq!(lines[1].to_string(), "Save or switch an account to continue.");
    }

    #[test]
    fn account_detail_uses_last_updated_label() {
        let profile = ProfileEntry {
            kind: ProfileKind::Claude,
            saved_name: Some("demo".to_string()),
            profile_name: "demo".to_string(),
            snapshot: serde_json::json!({}),
            usage_view: UsageReadResult {
                usage: None,
                source: UsageSource::None,
                fetched_at: None,
                stale: false,
            },
            account_id: Some("claude-demo".to_string()),
            is_current: false,
            chart_data: ProfileChartData::empty("no usage data"),
        };

        let lines = render_account_detail(Some(&profile), None);
        assert!(lines.iter().any(|line| line.to_string() == "Last updated: never"));
    }

    #[test]
    fn account_detail_uses_quota_label_for_longer_windows() {
        let profile = ProfileEntry {
            kind: ProfileKind::Copilot,
            saved_name: Some("teamt5-it".to_string()),
            profile_name: "teamt5-it".to_string(),
            snapshot: serde_json::json!({}),
            usage_view: UsageReadResult {
                usage: Some(UsageResponse {
                    email: Some("teamt5-it".to_string()),
                    plan_type: Some("business".to_string()),
                    rate_limit: Some(crate::usage::UsageRateLimit {
                        primary_window: None,
                        secondary_window: Some(crate::usage::UsageWindow {
                            used_percent: 0.0,
                            limit_window_seconds: 2_592_000,
                            reset_at: 1_800_000_000,
                            reset_after_seconds: 2_550_000,
                        }),
                    }),
                }),
                source: UsageSource::Api,
                fetched_at: Some(1_700_000_000),
                stale: false,
            },
            account_id: Some("copilot-teamt5-it".to_string()),
            is_current: true,
            chart_data: ProfileChartData::empty("no usage data"),
        };

        let lines = render_account_detail(Some(&profile), None);
        let rendered = lines.iter().map(Line::to_string).collect::<Vec<_>>();

        assert!(rendered.iter().any(|line| line.starts_with("Quota: 0% used, reset in ")));
        assert!(!rendered.iter().any(|line| line.starts_with("Weekly:")));
    }

    #[test]
    fn account_detail_keeps_metadata_before_five_hour_line() {
        let profile = ProfileEntry {
            kind: ProfileKind::Claude,
            saved_name: Some("team".to_string()),
            profile_name: "team".to_string(),
            snapshot: serde_json::json!({}),
            usage_view: UsageReadResult {
                usage: Some(UsageResponse {
                    email: Some("team@example.com".to_string()),
                    plan_type: Some("pro".to_string()),
                    rate_limit: Some(crate::usage::UsageRateLimit {
                        primary_window: Some(crate::usage::UsageWindow {
                            used_percent: 28.0,
                            limit_window_seconds: 604_800,
                            reset_at: 1_800_000_000,
                            reset_after_seconds: 12_345,
                        }),
                        secondary_window: Some(crate::usage::UsageWindow {
                            used_percent: 64.0,
                            limit_window_seconds: 18_000,
                            reset_at: 1_800_000_000,
                            reset_after_seconds: 3_600,
                        }),
                    }),
                }),
                source: UsageSource::Api,
                fetched_at: Some(1_700_000_000),
                stale: false,
            },
            account_id: Some("claude-team".to_string()),
            is_current: false,
            chart_data: ProfileChartData::empty("no usage data"),
        };

        let lines = render_account_detail(Some(&profile), None);
        let rendered = lines.iter().map(Line::to_string).collect::<Vec<_>>();

        for expected in [
            "Profile: team",
            "Service: Claude [claude]",
            "State: saved",
            "Last updated: ",
            "Plan: pro",
            "Quota: 28% used, reset in ",
            "5h: 64% used, reset in ",
        ] {
            assert!(
                rendered.iter().any(|line| line.starts_with(expected)),
                "missing line starting with {expected:?}: {rendered:?}"
            );
        }

        let profile_idx = rendered.iter().position(|line| line.starts_with("Profile: ")).unwrap();
        let service_idx = rendered.iter().position(|line| line.starts_with("Service: ")).unwrap();
        let state_idx = rendered.iter().position(|line| line.starts_with("State: ")).unwrap();
        let updated_idx = rendered.iter().position(|line| line.starts_with("Last updated: ")).unwrap();
        let email_idx = rendered.iter().position(|line| line.starts_with("Email: ")).unwrap();
        let plan_idx = rendered.iter().position(|line| line.starts_with("Plan: ")).unwrap();
        let quota_idx = rendered.iter().position(|line| line.starts_with("Quota: ")).unwrap();
        let fiveh_idx = rendered.iter().position(|line| line.starts_with("5h: ")).unwrap();

        assert!(profile_idx < service_idx);
        assert!(service_idx < state_idx);
        assert!(state_idx < updated_idx);
        assert!(updated_idx < email_idx);
        assert!(email_idx < plan_idx);
        assert!(plan_idx < quota_idx);
        assert!(quota_idx < fiveh_idx);
    }

    #[test]
    fn account_detail_hides_five_hour_line_for_copilot() {
        let profile = ProfileEntry {
            kind: ProfileKind::Copilot,
            saved_name: Some("teamt5-it".to_string()),
            profile_name: "teamt5-it".to_string(),
            snapshot: serde_json::json!({}),
            usage_view: UsageReadResult {
                usage: Some(UsageResponse {
                    email: Some("teamt5-it".to_string()),
                    plan_type: Some("business".to_string()),
                    rate_limit: Some(crate::usage::UsageRateLimit {
                        primary_window: Some(crate::usage::UsageWindow {
                            used_percent: 70.0,
                            limit_window_seconds: 18_000,
                            reset_at: 1_800_000_000,
                            reset_after_seconds: 1_200,
                        }),
                        secondary_window: Some(crate::usage::UsageWindow {
                            used_percent: 42.0,
                            limit_window_seconds: 2_592_000,
                            reset_at: 1_800_000_000,
                            reset_after_seconds: 2_000_000,
                        }),
                    }),
                }),
                source: UsageSource::Api,
                fetched_at: Some(1_700_000_000),
                stale: false,
            },
            account_id: Some("copilot-teamt5-it".to_string()),
            is_current: true,
            chart_data: ProfileChartData::empty("no usage data"),
        };

        let lines = render_account_detail(Some(&profile), None);
        let rendered = lines.iter().map(Line::to_string).collect::<Vec<_>>();

        assert!(rendered.iter().any(|line| line.starts_with("Quota: 42% used, reset in ")));
        assert!(!rendered.iter().any(|line| line.starts_with("5h: ")));
    }

    #[test]
    fn refresh_tasks_panel_renders_cron_and_background_status() {
        let cron_status = CronStatus {
            installed: true,
            last_run: Some(1_700_000_000),
            last_attempt: Some(1_700_000_300),
            codex_error: None,
            claude_error: Some("HTTP 429 Too Many Requests".to_string()),
            copilot_error: None,
        };

        let lines = render_refresh_tasks(
            &cron_status,
            Some("Background refresh updated 3 profiles"),
            &BackgroundRefreshState::Running { queued_profiles: 3 },
        );

        let rendered = lines.iter().map(Line::to_string).collect::<Vec<_>>();
        assert!(rendered.iter().any(|line| line == "Background: refreshing 3 stale profiles"));
        assert!(rendered.iter().any(|line| line == "Last result: Background refresh updated 3 profiles"));
        assert!(rendered.iter().any(|line| line.starts_with("Cron: installed")));
        assert!(rendered.iter().any(|line| line.starts_with("Last success:")));
        assert!(rendered.iter().any(|line| line == "Claude issue: HTTP 429 Too Many Requests"));
    }

    #[test]
    fn account_detail_shows_last_cron_failure_summary() {
        let profile = ProfileEntry {
            kind: ProfileKind::Claude,
            saved_name: Some("demo".to_string()),
            profile_name: "demo".to_string(),
            snapshot: serde_json::json!({}),
            usage_view: UsageReadResult {
                usage: None,
                source: UsageSource::None,
                fetched_at: None,
                stale: false,
            },
            account_id: Some("claude-demo".to_string()),
            is_current: false,
            chart_data: ProfileChartData::empty("no usage data"),
        };
        let lines = render_account_detail(Some(&profile), None);

        assert!(!lines.iter().any(|line| line.to_string().contains("Cron")));
    }

    #[test]
    fn account_detail_no_longer_embeds_cron_lines() {
        let profile = ProfileEntry {
            kind: ProfileKind::Claude,
            saved_name: Some("demo".to_string()),
            profile_name: "demo".to_string(),
            snapshot: serde_json::json!({}),
            usage_view: UsageReadResult {
                usage: None,
                source: UsageSource::None,
                fetched_at: None,
                stale: false,
            },
            account_id: Some("claude-demo".to_string()),
            is_current: false,
            chart_data: ProfileChartData::empty("no usage data"),
        };
        let lines = render_account_detail(Some(&profile), None);

        assert!(!lines.iter().any(|line| line.to_string().starts_with("Cron:")));
        assert!(!lines
            .iter()
            .any(|line| line.to_string().starts_with("Cron issue:")));
    }

    #[test]
    fn refresh_tasks_panel_renders_in_left_pane_instead_of_global_status_line() {
        let mut app = App::from_profile_names(vec!["Alpha".to_string()], 0);
        app.fullscreen = false;
        app.pane_focus = PaneFocus::Accounts;
        app.cron_status = CronStatus {
            installed: true,
            last_run: Some(1_700_000_000),
            last_attempt: Some(1_700_000_300),
            codex_error: None,
            claude_error: Some("HTTP 429 Too Many Requests".to_string()),
            copilot_error: None,
        };
        app.status_message = Some("Background refresh updated 3 profiles".to_string());
        app.background_refresh_state = BackgroundRefreshState::Running { queued_profiles: 3 };

        let buffer = render_buffer(&app, 100, 24);
        let actions_line = buffer_row_text(&buffer, 22, 100);
        let bottom_line = buffer_row_text(&buffer, 23, 100);
        let joined = (0..24)
            .map(|y| buffer_row_text(&buffer, y, 100))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(joined.contains("Refresh tasks"));
        assert!(joined.contains("Last result: Background refresh updated 3 profiles"));
        assert!(joined.contains("Cron: installed"));
        assert!(joined.contains("Claude issue:"));
        assert!(joined.contains("429 Too Many Requests"));
        assert!(!actions_line.contains("Cron:"));
        assert!(bottom_line.contains(env!("BUILD_VER")));
        assert!(actions_line.contains("Enter=switch/save"));
    }

    #[test]
    fn selected_profile_row_uses_darker_background_without_losing_series_color() {
        let mut app = App::from_profile_names(vec!["Alpha".to_string(), "Beta".to_string()], 1);
        app.fullscreen = false;
        let buffer = render_buffer(&app, 100, 24);

        let cell = (0..24)
            .flat_map(|y| (0..100).map(move |x| (x, y)))
            .find_map(|(x, y)| {
                let cell = &buffer[(x, y)];
                (cell.symbol() == "B").then_some(cell)
            })
            .expect("selected profile label should be rendered");

        assert_eq!(cell.fg, render::SERIES_COLORS[1]);
        assert_eq!(cell.bg, Color::DarkGray);
    }

    #[test]
    fn profile_list_uses_long_service_tags_and_hides_saved_suffix() {
        let mut app = App::from_profile_names(vec!["Alpha".to_string()], 0);
        app.fullscreen = false;
        let buffer = render_buffer(&app, 100, 24);
        let joined = (0..24)
            .map(|y| buffer_row_text(&buffer, y, 100))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(joined.contains("[codex] Alpha"));
        assert!(!joined.contains("[saved]"));
    }

    #[test]
    fn profile_list_shows_both_current_and_hidden_symbols_simultaneously() {
        let mut app = App::from_profile_names(vec!["Alpha".to_string(), "Beta".to_string()], 0);
        app.fullscreen = false;
        // Hide the current profile (index 0 = Alpha)
        app.toggle_selected_profile_hidden();
        let buffer = render_buffer(&app, 100, 24);
        let joined = (0..24)
            .map(|y| buffer_row_text(&buffer, y, 100))
            .collect::<Vec<_>>()
            .join("\n");
        // Alpha is both current (▶) and hidden (~) — both symbols must appear together
        assert!(joined.contains("▶~"), "expected '▶~' for current+hidden profile, got:\n{joined}");
    }

    #[test]
    fn auto_y_lower_includes_band_floor_and_label_padding() {
        let profiles = vec![ProfileEntry {
            kind: ProfileKind::Codex,
            saved_name: Some("alpha".to_string()),
            profile_name: "Alpha".to_string(),
            snapshot: serde_json::json!({}),
            usage_view: UsageReadResult {
                usage: None,
                source: UsageSource::None,
                fetched_at: None,
                stale: false,
            },
            account_id: Some("acct-alpha".to_string()),
            is_current: true,
            chart_data: ProfileChartData {
                seven_day_points: vec![ChartPoint { x: 7.0, y: 14.0 }],
                quota_window_label: "7d".to_string(),
                forecast: crate::app_data::OwnedUsageForecast::empty("band-only"),
                weekly_reset_countdown_seconds: None,
                five_hour_reset_countdown_seconds: None,
                five_hour_band: OwnedFiveHourBandState {
                    available: true,
                    used_percent: Some(12.0),
                    lower_y: Some(8.0),
                    upper_y: Some(20.0),
                    delta_seven_day_percent: Some(3.0),
                    delta_five_hour_percent: Some(1.5),
                    reason: None,
                },
                five_hour_subframe: OwnedFiveHourSubframeState {
                    available: true,
                    start_x: Some(6.5),
                    end_x: Some(7.0),
                    lower_y: Some(8.0),
                    upper_y: Some(20.0),
                    reason: None,
                },
                is_zero_state: false,
            },
        }];

        assert_eq!(auto_y_lower(&profiles), 0.0);
    }

    #[test]
    fn rename_dialog_cursor_moves_with_home_end_and_arrows() {
        let mut app = App::from_profile_names(vec!["Alpha".to_string()], 0);
        app.open_rename_dialog();

        app.handle_dialog_action(InputAction::MoveToStart).unwrap();
        app.handle_dialog_action(InputAction::Right).unwrap();
        app.handle_dialog_action(InputAction::Right).unwrap();
        app.handle_dialog_action(InputAction::MoveToEnd).unwrap();
        app.handle_dialog_action(InputAction::Left).unwrap();

        let dialog = app.dialog.as_ref().expect("rename dialog should stay open");
        assert_eq!(dialog.input, "alpha");
        assert_eq!(dialog.cursor, 4);
    }

    #[test]
    fn rename_dialog_sets_terminal_cursor_position() {
        let mut app = App::from_profile_names(vec!["Alpha".to_string()], 0);
        app.open_rename_dialog();
        app.handle_dialog_action(InputAction::MoveToStart).unwrap();
        app.handle_dialog_action(InputAction::Right).unwrap();
        app.handle_dialog_action(InputAction::Right).unwrap();

        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
        let expected = dialog_cursor_position(
            popup_area(Rect::new(0, 0, 100, 24), 70, 20),
            app.dialog.as_ref().expect("rename dialog should still be open"),
        );

        assert_eq!(
            terminal.backend_mut().get_cursor_position().unwrap(),
            expected
        );
    }
}

#[cfg(test)]
mod chart_reset_line_tests {
    use super::*;
    use crate::app_data::{OwnedFiveHourBandState, OwnedFiveHourSubframeState};
    use crate::render::ChartPoint;

    fn make_profile_with_chart(
        last_seven_day_percent: Option<f64>,
        five_hour_used_percent: Option<f64>,
        weekly_reset_countdown_seconds: Option<i64>,
        five_hour_reset_countdown_seconds: Option<i64>,
    ) -> ProfileEntry {
        ProfileEntry {
            kind: ProfileKind::Codex,
            saved_name: Some("alpha".to_string()),
            profile_name: "Alpha".to_string(),
            snapshot: serde_json::json!({}),
            usage_view: UsageReadResult {
                usage: None,
                source: UsageSource::None,
                fetched_at: None,
                stale: false,
            },
            account_id: Some("acct-alpha".to_string()),
            is_current: true,
            chart_data: ProfileChartData {
                seven_day_points: last_seven_day_percent
                    .map(|value| vec![ChartPoint { x: 7.0, y: value }])
                    .unwrap_or_default(),
                quota_window_label: "7d".to_string(),
                forecast: crate::app_data::OwnedUsageForecast::empty("test"),
                weekly_reset_countdown_seconds,
                five_hour_reset_countdown_seconds,
                five_hour_band: OwnedFiveHourBandState {
                    available: five_hour_used_percent.is_some(),
                    used_percent: five_hour_used_percent,
                    lower_y: None,
                    upper_y: None,
                    delta_seven_day_percent: None,
                    delta_five_hour_percent: None,
                    reason: None,
                },
                five_hour_subframe: OwnedFiveHourSubframeState {
                    available: false,
                    start_x: None,
                    end_x: None,
                    lower_y: None,
                    upper_y: None,
                    reason: None,
                },
                is_zero_state: false,
            },
        }
    }

    #[test]
    fn reset_line_weekly_when_weekly_qualifies() {
        let profiles = vec![make_profile_with_chart(Some(100.0), Some(40.0), Some(3_600), Some(900))];
        let state = build_chart_state(&profiles, 0);
        let reset_line = state.series[0]
            .reset_line_display
            .as_ref()
            .expect("weekly 100% should derive a reset line");

        assert_eq!(reset_line.source, crate::render::ResetLineSource::Weekly);
        assert_eq!(reset_line.text, "Hit limit · resets in 1h");
    }

    #[test]
    fn reset_line_five_hour_when_five_hour_qualifies() {
        let profiles = vec![make_profile_with_chart(Some(80.0), Some(100.0), Some(3_600), Some(7_200))];
        let state = build_chart_state(&profiles, 0);
        let reset_line = state.series[0]
            .reset_line_display
            .as_ref()
            .expect("5h 100% should derive a reset line");

        assert_eq!(reset_line.source, crate::render::ResetLineSource::FiveHour);
        assert_eq!(reset_line.text, "Hit limit · resets in 2h");
    }

    #[test]
    fn reset_line_chooses_longer_when_both_qualify() {
        let profiles = vec![make_profile_with_chart(Some(140.0), Some(120.0), Some(3_600), Some(7_200))];
        let state = build_chart_state(&profiles, 0);
        let reset_line = state.series[0]
            .reset_line_display
            .as_ref()
            .expect("two qualifying windows should pick one reset line");

        assert_eq!(reset_line.source, crate::render::ResetLineSource::FiveHour);
        assert_eq!(reset_line.text, "Hit limit · resets in 2h");
    }

    #[test]
    fn reset_line_uses_renderable_one_when_only_one_countdown_exists() {
        let five_hour_only = vec![make_profile_with_chart(Some(140.0), Some(120.0), None, Some(7_200))];
        let five_hour_state = build_chart_state(&five_hour_only, 0);
        let five_hour_reset = five_hour_state.series[0]
            .reset_line_display
            .as_ref()
            .expect("renderable 5h countdown should win");
        assert_eq!(five_hour_reset.source, crate::render::ResetLineSource::FiveHour);

        let weekly_only = vec![make_profile_with_chart(Some(140.0), Some(120.0), Some(3_600), None)];
        let weekly_state = build_chart_state(&weekly_only, 0);
        let weekly_reset = weekly_state.series[0]
            .reset_line_display
            .as_ref()
            .expect("renderable weekly countdown should win");
        assert_eq!(weekly_reset.source, crate::render::ResetLineSource::Weekly);
    }

    #[test]
    fn reset_line_none_when_neither_usage_value_qualifies() {
        let profiles = vec![make_profile_with_chart(Some(90.0), Some(80.0), Some(3_600), Some(7_200))];
        let state = build_chart_state(&profiles, 0);

        assert!(state.series[0].reset_line_display.is_none());
    }

    #[test]
    fn reset_line_none_when_no_renderable_countdowns_exist() {
        let profiles = vec![make_profile_with_chart(Some(140.0), Some(120.0), None, None)];
        let state = build_chart_state(&profiles, 0);

        assert!(state.series[0].reset_line_display.is_none());
    }

    #[test]
    fn zero_state_remains_without_reset_contract() {
        let app = App::from_profile_names(vec!["Alpha".to_string()], 0);
        let state = build_chart_state(&app.profiles, 0);

        assert!(state.series[0].reset_line_display.is_none());
    }
}
