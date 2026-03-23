use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use crossterm::event;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::prelude::*;
use ratatui::style::{Modifier, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use serde_json::Value;

use crate::cron::CronStatus;
use crate::input::{self, InputAction};
use crate::render;
use crate::render::{
    ChartPoint, ChartSeries, ChartSeriesStyle, ChartState, FiveHourBandState, FiveHourSubframeState,
    RenderProfile, SelectionState,
};
use crate::store::{AccountStore, SavedProfile};
use crate::usage::{
    UsageReadResult, UsageResponse, UsageService, UsageSource, UsageWindow, UsageWindowHistory,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfileKind {
    Codex,
    Claude,
}

#[derive(Debug, Clone)]
struct ProfileEntry {
    kind: ProfileKind,
    saved_name: Option<String>,
    profile_name: String,
    snapshot: Value,
    usage_view: UsageReadResult,
    account_id: Option<String>,
    is_current: bool,
    chart_data: ProfileChartData,
}

#[derive(Debug, Clone, PartialEq)]
struct ProfileChartData {
    seven_day_points: Vec<ChartPoint>,
    five_hour_band: OwnedFiveHourBandState,
    five_hour_subframe: OwnedFiveHourSubframeState,
}

#[derive(Debug, Clone, PartialEq)]
struct OwnedFiveHourBandState {
    available: bool,
    lower_y: Option<f64>,
    upper_y: Option<f64>,
    delta_seven_day_percent: Option<f64>,
    delta_five_hour_percent: Option<f64>,
    reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct OwnedFiveHourSubframeState {
    available: bool,
    start_x: Option<f64>,
    end_x: Option<f64>,
    lower_y: Option<f64>,
    upper_y: Option<f64>,
    reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DialogMode {
    SaveCurrent(ProfileKind),
    RenameSaved(String),
    ConfirmDelete(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogState {
    mode: DialogMode,
    input: String,
}

pub struct App {
    profiles: Vec<ProfileEntry>,
    selected_profile_index: usize,
    pane_focus: PaneFocus,
    y_zoom_lower: f64,
    should_quit: bool,
    dialog: Option<DialogState>,
    status_message: Option<String>,
    store: Option<AccountStore>,
    usage_service: Option<UsageService>,
    claude_store: Option<crate::claude::ClaudeStore>,
    claude_usage_service: Option<UsageService>,
    cron_status: CronStatus,
    solo_mode: bool,
    x_window_days: u8,
    cursor_x: Option<f64>,
    filter_input: Option<String>,
    last_auto_reload: Instant,
    y_zoom_user_adjusted: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AppRenderState<'a> {
    profiles: &'a [ProfileEntry],
    selected_profile_index: usize,
    y_zoom_lower: f64,
    solo: bool,
    x_window_days: u8,
    cursor_x: Option<f64>,
}

impl ProfileChartData {
    fn empty(reason: &str) -> Self {
        Self {
            seven_day_points: Vec::new(),
            five_hour_band: OwnedFiveHourBandState {
                available: false,
                lower_y: None,
                upper_y: None,
                delta_seven_day_percent: None,
                delta_five_hour_percent: None,
                reason: Some(reason.to_string()),
            },
            five_hour_subframe: OwnedFiveHourSubframeState {
                available: false,
                start_x: None,
                end_x: None,
                lower_y: None,
                upper_y: None,
                reason: Some(reason.to_string()),
            },
        }
    }
}

impl App {
    pub fn load(
        store: AccountStore,
        usage_service: UsageService,
        cron_status: CronStatus,
        claude_store: Option<crate::claude::ClaudeStore>,
        claude_usage_service: Option<UsageService>,
    ) -> Result<Self> {
        let profiles = load_profiles(&store, &usage_service, false, None, claude_store.as_ref(), claude_usage_service.as_ref())?;
        Ok(Self {
            selected_profile_index: initial_selected_index(&profiles),
            y_zoom_lower: auto_y_lower(&profiles),
            profiles,
            pane_focus: PaneFocus::Accounts,
            should_quit: false,
            dialog: None,
            status_message: None,
            store: Some(store),
            usage_service: Some(usage_service),
            claude_store,
            claude_usage_service,
            cron_status,
            solo_mode: false,
            x_window_days: 7,
            cursor_x: None,
            filter_input: None,
            last_auto_reload: Instant::now(),
            y_zoom_user_adjusted: false,
        })
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
            pane_focus: PaneFocus::Accounts,
            y_zoom_lower: 0.0,
            should_quit: false,
            dialog: None,
            status_message: None,
            store: None,
            usage_service: None,
            claude_store: None,
            claude_usage_service: None,
            cron_status: CronStatus::uninstalled(),
            solo_mode: false,
            x_window_days: 7,
            cursor_x: None,
            filter_input: None,
            last_auto_reload: Instant::now(),
            y_zoom_user_adjusted: false,
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
        let mut terminal = TerminalSession::enter();
        terminal.run(self)
    }

    fn run_loop(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;

            if event::poll(Duration::from_millis(150))? {
                let event = event::read()?;
                if let Some(action) = input::map_event(&event) {
                    self.handle_action(action)?;
                }
            }

            // Auto-reload every 30s to pick up cron-refreshed usage data
            if self.last_auto_reload.elapsed() >= Duration::from_secs(30) {
                let _ = self.reload_profiles(false, None);
                self.last_auto_reload = Instant::now();
            }
        }
        Ok(())
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
                let delta = if matches!(action, InputAction::Up) { -1 } else { 1 };
                self.step_profile(delta);
            }
            InputAction::Left => {
                if self.pane_focus == PaneFocus::Plot {
                    self.move_cursor(-1);
                }
            }
            InputAction::Right => {
                if self.pane_focus == PaneFocus::Plot {
                    self.move_cursor(1);
                }
            }
            InputAction::NextFocus | InputAction::PreviousFocus => {
                self.pane_focus = self.pane_focus.toggle();
            }
            InputAction::ZoomIn => {
                self.y_zoom_lower = (self.y_zoom_lower + 5.0).min(95.0);
                self.y_zoom_user_adjusted = true;
            }
            InputAction::ZoomOut => {
                self.y_zoom_lower = (self.y_zoom_lower - 5.0).max(0.0);
                self.y_zoom_user_adjusted = true;
            }
            InputAction::ResetZoom => {
                self.y_zoom_lower = auto_y_lower(&self.profiles);
                self.y_zoom_user_adjusted = false;
            }
            InputAction::ToggleSolo => {
                if self.pane_focus == PaneFocus::Plot {
                    self.solo_mode = !self.solo_mode;
                }
            }
            InputAction::XWindow(days) => {
                if self.pane_focus == PaneFocus::Plot {
                    self.x_window_days = days;
                    if let Some(cx) = self.cursor_x {
                        let new_lower = 7.0 - days as f64;
                        self.cursor_x = Some(cx.clamp(new_lower, 7.0));
                    }
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
                    self.refresh_selected_profile(true)?;
                }
            }
            InputAction::RefreshAll => self.reload_profiles(true, None)?,
            InputAction::Backspace | InputAction::Character(_) | InputAction::Cancel => {}
        }

        Ok(())
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

    fn move_cursor(&mut self, direction: isize) {
        let x_lower = 7.0 - self.x_window_days as f64;
        let step = self.x_window_days as f64 / 20.0;
        let current = self.cursor_x.unwrap_or(7.0);
        let next = (current + direction as f64 * step).clamp(x_lower, 7.0);
        self.cursor_x = Some(next);
    }

    fn handle_dialog_action(&mut self, action: InputAction) -> Result<()> {
        match action {
            InputAction::Quit | InputAction::Cancel => {
                self.dialog = None;
            }
            InputAction::Backspace => {
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.input.pop();
                }
            }
            InputAction::Character(ch) => {
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.input.push(ch);
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
                };
                self.status_message = Some(format!("Saved current profile as \"{name}\"."));
                self.dialog = None;
                self.reload_profiles(false, profile.account_id.clone())?;
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
                self.reload_profiles(false, None)?;
            }
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
                    self.reload_profiles(false, None)?;
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
                        self.reload_profiles(false, profile.account_id.clone())?;
                    }
                } else {
                    self.dialog = Some(DialogState {
                        mode: DialogMode::SaveCurrent(ProfileKind::Claude),
                        input: build_default_name(profile.usage_view.usage.as_ref(), &profile.snapshot),
                    });
                }
            }
            ProfileKind::Codex => {
                if let Some(saved_name) = profile.saved_name.as_deref() {
                    if let Some(store) = self.store.as_ref() {
                        let activated = store.use_account(saved_name)?;
                        self.status_message = Some(format!("Switched Codex auth to \"{activated}\"."));
                        self.reload_profiles(false, profile.account_id.clone())?;
                    }
                } else {
                    self.dialog = Some(DialogState {
                        mode: DialogMode::SaveCurrent(ProfileKind::Codex),
                        input: build_default_name(profile.usage_view.usage.as_ref(), &profile.snapshot),
                    });
                }
            }
        }
        Ok(())
    }

    fn open_rename_dialog(&mut self) {
        let Some(profile) = self.selected_profile() else {
            return;
        };
        let Some(saved_name) = profile.saved_name.clone() else {
            return;
        };
        self.dialog = Some(DialogState {
            mode: DialogMode::RenameSaved(saved_name.clone()),
            input: saved_name,
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
        });
    }

    fn refresh_selected_profile(&mut self, force_refresh: bool) -> Result<()> {
        let account_id = self.selected_profile().and_then(|profile| profile.account_id.clone());
        self.reload_profiles(force_refresh, account_id)
    }

    fn reload_profiles(&mut self, force_refresh: bool, refresh_account_id: Option<String>) -> Result<()> {
        let (Some(store), Some(usage_service)) = (self.store.as_ref(), self.usage_service.as_ref()) else {
            return Ok(());
        };
        self.profiles = load_profiles(
            store,
            usage_service,
            force_refresh,
            refresh_account_id.as_deref(),
            self.claude_store.as_ref(),
            self.claude_usage_service.as_ref(),
        )?;
        self.selected_profile_index = self
            .selected_profile_index
            .min(self.profiles.len().saturating_sub(1));
        if !self.y_zoom_user_adjusted {
            self.y_zoom_lower = auto_y_lower(&self.profiles);
        }
        Ok(())
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
        // "  " highlight_symbol + "▶ {name}{unsaved_tag} {badge}" + 2 border cols
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
                render_account_detail(Some(profile), None, &self.cron_status)
            })
            .map(|line| line.width())
            .max()
            .unwrap_or(0) + 2; // +2 for "Details" block borders

        let max_content = max_list.max(max_detail) as u16;
        // Give the chart at least 40 columns; always at least 20 for the left pane.
        let max_allowed = total_width.saturating_sub(40).max(20);
        max_content.min(max_allowed)
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let [body, footer_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).areas(area);

        let left_width = self.left_pane_width(body.width);
        let [left_area, right_area] =
            Layout::horizontal([Constraint::Length(left_width), Constraint::Min(0)]).areas(body);

        self.render_left_pane(frame, left_area);

        let render_state = AppRenderState {
            profiles: &self.profiles,
            selected_profile_index: self.selected_profile_index,
            y_zoom_lower: self.y_zoom_lower,
            solo: self.solo_mode,
            x_window_days: self.x_window_days,
            cursor_x: self.cursor_x,
        };
        render::render(frame, right_area, &render_state);

        let footer_lines = match self.pane_focus {
            PaneFocus::Accounts => vec![
                Line::from("Enter=switch · n=rename · d=delete · u=refresh · a=all · q=quit"),
                Line::from(format!(
                    "Tab=plot · ↑↓=navigate · /=filter{}",
                    if self.filter_input.is_some() { " (Esc=clear)" } else { "" }
                )),
            ],
            PaneFocus::Plot => vec![
                Line::from(format!(
                    "1/3/7=window · s=solo{} · +/-=zoom · r=reset · a=refresh · q=quit",
                    if self.solo_mode { "[ON]" } else { "" }
                )),
                Line::from("Tab=accounts · ↑↓=profile · ←→=cursor"),
            ],
        };
        let footer = Paragraph::new(Text::from(footer_lines)).wrap(Wrap { trim: true });
        frame.render_widget(footer, footer_area);

        if self.dialog.is_some() {
            self.render_dialog(frame);
        }
    }

    fn render_left_pane(&self, frame: &mut Frame, area: Rect) {
        let indices = self.filtered_profile_indices();
        let list_lines = (indices.len().max(3).min(10) + 2) as u16;

        let [list_area, detail_area] =
            Layout::vertical([Constraint::Length(list_lines), Constraint::Min(0)]).areas(area);

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
                let prefix = if profile.is_current { "▶ " } else { "  " };
                let unsaved_tag = if profile.saved_name.is_none() { " [unsaved]" } else { "" };
                let usage = format_usage_badge(&profile.usage_view);
                let label = format!("{}{}{unsaved_tag}", prefix, profile.profile_name);
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
        let list = List::new(items)
            .block(Block::default().title(profiles_title).borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, list_area, &mut state);

        let detail_lines =
            render_account_detail(self.selected_profile(), self.status_message.as_deref(), &self.cron_status);
        let details = Paragraph::new(Text::from(detail_lines))
            .block(Block::default().title("Details").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(details, detail_area);
    }

    fn render_dialog(&self, frame: &mut Frame) {
        let Some(dialog) = self.dialog.as_ref() else {
            return;
        };
        let title = match &dialog.mode {
            DialogMode::SaveCurrent(_) => "Save current profile",
            DialogMode::RenameSaved(_) => "Rename saved profile",
            DialogMode::ConfirmDelete(_) => "Delete saved profile",
        };
        let prompt = match &dialog.mode {
            DialogMode::SaveCurrent(_) => "Enter a name for the current auth snapshot.",
            DialogMode::RenameSaved(_) => "Enter the new saved profile name.",
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
    }
}

impl render::RenderState for AppRenderState<'_> {
    fn selection_state(&self) -> SelectionState<'_> {
        SelectionState {
            selected: self.selected_profile().map(|profile| RenderProfile {
                id: profile.account_id.as_deref().unwrap_or(profile.profile_name.as_str()),
                label: profile.profile_name.as_str(),
                is_current: profile.is_current,
            }),
            current: self.current_profile().map(|profile| RenderProfile {
                id: profile.account_id.as_deref().unwrap_or(profile.profile_name.as_str()),
                label: profile.profile_name.as_str(),
                is_current: profile.is_current,
            }),
        }
    }

    fn chart_state(&self) -> ChartState<'_> {
        let mut state = build_chart_state(self.profiles, self.selected_profile_index);
        state.y_lower = self.y_zoom_lower;
        state.y_upper = 100.0;
        state.x_lower = 7.0 - self.x_window_days as f64;
        state.solo = self.solo;
        state.cursor_x = self.cursor_x;
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

fn load_profiles(
    store: &AccountStore,
    usage_service: &UsageService,
    force_refresh: bool,
    refresh_account_id: Option<&str>,
    claude_store: Option<&crate::claude::ClaudeStore>,
    claude_usage_service: Option<&UsageService>,
) -> Result<Vec<ProfileEntry>> {
    let saved_profiles = store.list_saved_profiles()?;
    let current_snapshot = store.get_current_snapshot().ok();
    let current_account_id = current_snapshot.as_ref().and_then(read_account_id);
    let current_access_token = current_snapshot.as_ref().and_then(read_access_token);

    let mut profiles = saved_profiles
        .into_iter()
        .map(|profile| build_saved_entry(profile, &current_account_id, usage_service, force_refresh, refresh_account_id))
        .collect::<Result<Vec<_>>>()?;

    if let Some(snapshot) = current_snapshot {
        let current_saved = current_account_id.as_ref().is_some_and(|account_id| {
            profiles
                .iter()
                .any(|profile| profile.account_id.as_deref() == Some(account_id.as_str()))
        });
        if !current_saved {
            let force_current = refresh_account_id
                .is_some_and(|account_id| current_account_id.as_deref() == Some(account_id))
                || force_refresh;
            let usage_view = usage_service.read_usage(
                current_account_id.as_deref(),
                current_access_token.as_deref(),
                force_current,
                false,
            )?;
            usage_service.record_usage_snapshot(current_account_id.as_deref(), usage_view.usage.as_ref())?;
            let chart_data = build_profile_chart_data(
                current_account_id.as_deref(),
                usage_view.usage.as_ref(),
                usage_service,
            )?;
            profiles.push(ProfileEntry {
                kind: ProfileKind::Codex,
                saved_name: None,
                profile_name: format!(
                    "{} [UNSAVED]",
                    build_default_name(usage_view.usage.as_ref(), &snapshot)
                ),
                account_id: current_account_id.clone(),
                is_current: true,
                snapshot,
                usage_view,
                chart_data,
            });
        }
    }

    // --- Claude entries ---
    if let (Some(cs), Some(cu)) = (claude_store, claude_usage_service) {
        let claude_entries = load_claude_profiles(cs, cu, force_refresh, refresh_account_id)?;
        profiles.extend(claude_entries);
    }

    profiles.sort_by(|left, right| left.profile_name.cmp(&right.profile_name));
    Ok(profiles)
}

fn load_claude_profiles(
    store: &crate::claude::ClaudeStore,
    usage_service: &UsageService,
    force_refresh: bool,
    refresh_account_id: Option<&str>,
) -> Result<Vec<ProfileEntry>> {
    let saved = store.list_saved_profiles()?;
    let current_creds = store.get_current_credentials().ok();
    let current_account_id = current_creds.as_ref().map(|c| c.account_id());

    // Helper: composite key for UsageService cache
    let composite_id = |creds: &crate::claude::ClaudeCredentials| {
        format!("{}|{}", creds.account_id(), creds.subscription_type())
    };

    let mut profiles: Vec<ProfileEntry> = saved
        .into_iter()
        .map(|saved_profile| {
            let snapshot = &saved_profile.snapshot;
            let creds: Option<crate::claude::ClaudeCredentials> =
                serde_json::from_value(snapshot.clone()).ok();
            let (acct_id, access_tok, comp_id) = match &creds {
                Some(c) => (
                    Some(c.account_id()),
                    Some(c.access_token().to_string()),
                    Some(composite_id(c)),
                ),
                None => (None, None, None),
            };
            let force_this = force_refresh
                || refresh_account_id.is_some_and(|t| acct_id.as_deref() == Some(t));
            let usage_view = usage_service.read_usage(
                comp_id.as_deref(),
                access_tok.as_deref(),
                force_this,
                false,
            )?;
            usage_service.record_usage_snapshot(comp_id.as_deref(), usage_view.usage.as_ref())?;
            let chart_data = build_profile_chart_data(
                comp_id.as_deref(),
                usage_view.usage.as_ref(),
                usage_service,
            )?;
            Ok(ProfileEntry {
                kind: ProfileKind::Claude,
                saved_name: Some(saved_profile.name.clone()),
                profile_name: saved_profile.name,
                snapshot: saved_profile.snapshot,
                usage_view,
                account_id: acct_id.clone(),
                is_current: current_account_id.as_deref() == acct_id.as_deref(),
                chart_data,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // Add unsaved current Claude profile if not already in saved list
    if let Some(creds) = &current_creds {
        let acct_id = creds.account_id();
        let access_tok = creds.access_token().to_string();
        let comp_id = composite_id(creds);
        let sub_type = creds.subscription_type().to_string();
        let already_saved = profiles.iter().any(|p| p.account_id.as_deref() == Some(acct_id.as_str()));
        if !already_saved {
            let force_current = force_refresh
                || refresh_account_id.is_some_and(|t| t == acct_id.as_str());
            let snapshot = store.get_current_snapshot().unwrap_or(serde_json::json!({}));
            let usage_view = usage_service.read_usage(
                Some(&comp_id),
                Some(access_tok.as_str()),
                force_current,
                false,
            )?;
            usage_service.record_usage_snapshot(Some(&comp_id), usage_view.usage.as_ref())?;
            let chart_data = build_profile_chart_data(
                Some(&comp_id),
                usage_view.usage.as_ref(),
                usage_service,
            )?;
            profiles.push(ProfileEntry {
                kind: ProfileKind::Claude,
                saved_name: None,
                profile_name: format!("{} [cl-unsaved]", sub_type),
                snapshot,
                usage_view,
                account_id: Some(acct_id),
                is_current: true,
                chart_data,
            });
        }
    }

    Ok(profiles)
}

fn build_saved_entry(
    profile: SavedProfile,
    current_account_id: &Option<String>,
    usage_service: &UsageService,
    force_refresh: bool,
    refresh_account_id: Option<&str>,
) -> Result<ProfileEntry> {
    let account_id = read_account_id(&profile.snapshot);
    let access_token = read_access_token(&profile.snapshot);
    let force_this_profile =
        force_refresh || refresh_account_id.is_some_and(|target| account_id.as_deref() == Some(target));
    let usage_view = usage_service.read_usage(
        account_id.as_deref(),
        access_token.as_deref(),
        force_this_profile,
        false,
    )?;
    usage_service.record_usage_snapshot(account_id.as_deref(), usage_view.usage.as_ref())?;
    let chart_data = build_profile_chart_data(account_id.as_deref(), usage_view.usage.as_ref(), usage_service)?;

    Ok(ProfileEntry {
        kind: ProfileKind::Codex,
        saved_name: Some(profile.name.clone()),
        profile_name: profile.name,
        snapshot: profile.snapshot,
        usage_view,
        account_id: account_id.clone(),
        is_current: current_account_id.as_deref() == account_id.as_deref(),
        chart_data,
    })
}

fn initial_selected_index(profiles: &[ProfileEntry]) -> usize {
    profiles.iter().position(|profile| profile.is_current).unwrap_or(0)
}

fn render_account_detail(
    profile: Option<&ProfileEntry>,
    status_message: Option<&str>,
    cron_status: &CronStatus,
) -> Vec<Line<'static>> {
    let Some(profile) = profile else {
        return vec![
            Line::from("No Codex auth profile loaded."),
            Line::from("Save or switch an account to continue."),
        ];
    };

    let state_label = match (profile.is_current, profile.saved_name.is_some()) {
        (true, _) => "current",
        (false, true) => "saved",
        (false, false) => "unsaved",
    };

    let mut lines = vec![
        Line::from(format!("Profile: {}", profile.profile_name)),
        Line::from(format!("State:   {}", state_label)),
        Line::from(format!(
            "Updated: {}",
            format_age(profile.usage_view.fetched_at, profile.usage_view.stale)
        )),
    ];

    if let Some(usage) = profile.usage_view.usage.as_ref() {
        if let Some(email) = usage.email.as_deref() {
            lines.push(Line::from(format!("Email:   {}", email)));
        }
        if let Some(plan) = usage.plan_type.as_deref() {
            lines.push(Line::from(format!("Plan:    {}", plan)));
        }
        if let Some(w) = pick_weekly_window(usage) {
            lines.push(Line::from(format!(
                "Weekly:  {:.0}% used, reset in {}",
                w.used_percent, format_duration_short(w.reset_after_seconds)
            )));
        }
        if let Some(w) = pick_five_hour_window(usage) {
            lines.push(Line::from(format!(
                "5h:      {:.0}% used, reset in {}",
                w.used_percent, format_duration_short(w.reset_after_seconds)
            )));
        }
    }

    lines.push(Line::from(""));
    let cron_line = if cron_status.installed {
        let age = cron_status.last_run
            .map(|ts| format_age(Some(ts), false))
            .unwrap_or_else(|| "never".to_string());
        format!("Cron: installed · {}", age)
    } else {
        "Cron: not installed".to_string()
    };
    lines.push(Line::from(cron_line));

    if let Some(message) = status_message {
        lines.push(Line::from(""));
        lines.push(Line::from(message.to_string()));
    }

    lines
}

fn format_age(fetched_at: Option<i64>, stale: bool) -> String {
    let Some(ts) = fetched_at else {
        return "never".to_string();
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
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

/// Format a duration in seconds as "3d 9h", "9h 38m", "45m", etc.
/// Uses the two largest non-zero units.
fn format_duration_short(secs: i64) -> String {
    let secs = secs.max(0) as u64;
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        if hours > 0 {
            format!("{}d {}h", days, hours)
        } else {
            format!("{}d", days)
        }
    } else if hours > 0 {
        if mins > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}h", hours)
        }
    } else {
        format!("{}m", mins.max(1))
    }
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

fn read_access_token(snapshot: &Value) -> Option<String> {
    snapshot
        .get("tokens")
        .and_then(|value| value.get("access_token"))
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
fn auto_y_lower(profiles: &[ProfileEntry]) -> f64 {
    let min_y = profiles
        .iter()
        .flat_map(|p| p.chart_data.seven_day_points.iter())
        .map(|pt| pt.y)
        .fold(f64::INFINITY, f64::min);
    if min_y.is_infinite() {
        return 0.0;
    }
    (min_y / 5.0).floor() * 5.0
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
            lower_y: profile.chart_data.five_hour_band.lower_y,
            upper_y: profile.chart_data.five_hour_band.upper_y,
            delta_seven_day_percent: profile.chart_data.five_hour_band.delta_seven_day_percent,
            delta_five_hour_percent: profile.chart_data.five_hour_band.delta_five_hour_percent,
            reason: profile.chart_data.five_hour_band.reason.as_deref(),
        })
        .unwrap_or(FiveHourBandState {
            available: false,
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
        .map(|(index, profile)| ChartSeries {
            profile: RenderProfile {
                id: profile.account_id.as_deref().unwrap_or(profile.profile_name.as_str()),
                label: profile.profile_name.as_str(),
                is_current: profile.is_current,
            },
            style: ChartSeriesStyle {
                color_slot: index,
                is_selected: index == selected_profile_index,
                is_current: profile.is_current,
            },
            points: profile.chart_data.seven_day_points.clone(),
            five_hour_subframe: FiveHourSubframeState {
                available: profile.chart_data.five_hour_subframe.available,
                start_x: profile.chart_data.five_hour_subframe.start_x,
                end_x: profile.chart_data.five_hour_subframe.end_x,
                lower_y: profile.chart_data.five_hour_subframe.lower_y,
                upper_y: profile.chart_data.five_hour_subframe.upper_y,
                reason: profile.chart_data.five_hour_subframe.reason.as_deref(),
            },
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
        solo: false,
        cursor_x: None,
    };

    if chart_state.series.is_empty() && chart_state.seven_day_points.is_empty() {
        chart_state.five_hour_band.reason = Some(selected_label);
        chart_state.five_hour_subframe.reason = Some(selected_label);
    }

    chart_state
}

fn build_profile_chart_data(
    account_id: Option<&str>,
    usage: Option<&UsageResponse>,
    usage_service: &UsageService,
) -> Result<ProfileChartData> {
    let Some(usage) = usage else {
        return Ok(ProfileChartData::empty("no usage data"));
    };
    let Some(account_id) = account_id else {
        return Ok(ProfileChartData::empty("no account id"));
    };

    let history = usage_service.profile_history(Some(account_id))?;
    let weekly_window = pick_weekly_window(usage);
    let five_hour_window = pick_five_hour_window(usage);
    let weekly_history = weekly_window
        .and_then(|window| find_matching_window(&history.weekly_windows, window));
    let seven_day_points = weekly_history
        .map(project_history_points)
        .unwrap_or_default();
    let five_hour_band = build_five_hour_band(weekly_window, five_hour_window);
    let five_hour_subframe = build_five_hour_subframe(weekly_window, five_hour_window, weekly_history);

    Ok(ProfileChartData {
        seven_day_points,
        five_hour_band,
        five_hour_subframe,
    })
}

fn build_five_hour_band(
    weekly_window: Option<&UsageWindow>,
    five_hour_window: Option<&UsageWindow>,
) -> OwnedFiveHourBandState {
    let Some(five_hour_window) = five_hour_window else {
        return OwnedFiveHourBandState {
            available: false,
            lower_y: None,
            upper_y: None,
            delta_seven_day_percent: None,
            delta_five_hour_percent: None,
            reason: Some("no 5h window".to_string()),
        };
    };
    let used = five_hour_window.used_percent.clamp(0.0, 100.0);
    OwnedFiveHourBandState {
        available: true,
        lower_y: Some((used - 10.0).max(0.0)),
        upper_y: Some((used + 10.0).min(100.0)),
        delta_seven_day_percent: weekly_window.map(|weekly| used - weekly.used_percent),
        delta_five_hour_percent: Some(0.0),
        reason: None,
    }
}

fn build_five_hour_subframe(
    weekly_window: Option<&UsageWindow>,
    five_hour_window: Option<&UsageWindow>,
    weekly_history: Option<&UsageWindowHistory>,
) -> OwnedFiveHourSubframeState {
    let Some(weekly_window) = weekly_window else {
        return OwnedFiveHourSubframeState {
            available: false,
            start_x: None,
            end_x: None,
            lower_y: None,
            upper_y: None,
            reason: Some("no 7d window".to_string()),
        };
    };
    let Some(five_hour_window) = five_hour_window else {
        return OwnedFiveHourSubframeState {
            available: false,
            start_x: None,
            end_x: None,
            lower_y: None,
            upper_y: None,
            reason: Some("no 5h window".to_string()),
        };
    };
    let weekly_start = weekly_window.reset_at - weekly_window.limit_window_seconds;
    let weekly_duration = weekly_window.limit_window_seconds as f64;
    let five_hour_start = five_hour_window.reset_at - five_hour_window.limit_window_seconds;
    let start_x = (((five_hour_start - weekly_start) as f64) / weekly_duration * 7.0).clamp(0.0, 7.0);
    let end_x = (((five_hour_window.reset_at - weekly_start) as f64) / weekly_duration * 7.0).clamp(0.0, 7.0);

    let current_7d = weekly_window.used_percent.clamp(0.0, 100.0);
    let five_hour_used = five_hour_window.used_percent.clamp(0.0, 100.0);

    // lower_y: first 7d observation at or after the 5h window start timestamp
    let lower_y = weekly_history
        .and_then(|hist| {
            hist.observations
                .iter()
                .find(|obs| obs.observed_at >= five_hour_start)
                .map(|obs| obs.used_percent.clamp(0.0, 100.0))
        })
        .unwrap_or_else(|| (current_7d - five_hour_used).max(0.0));

    // upper_y: project 5h usage to 100% using the observed 7d growth rate
    // Formula: lower_y + (7d_delta / 5h_used%) * 100
    let seven_day_delta = (current_7d - lower_y).max(0.0);
    let upper_y = if five_hour_used > 0.0 {
        (lower_y + (seven_day_delta / five_hour_used) * 100.0).clamp(lower_y, 100.0)
    } else {
        lower_y
    };

    OwnedFiveHourSubframeState {
        available: true,
        start_x: Some(start_x),
        end_x: Some(end_x.max(start_x)),
        lower_y: Some(lower_y),
        upper_y: Some(upper_y),
        reason: None,
    }
}

fn find_matching_window<'a>(
    windows: &'a [UsageWindowHistory],
    window: &UsageWindow,
) -> Option<&'a UsageWindowHistory> {
    let start_at = window.reset_at - window.limit_window_seconds;
    windows.iter().find(|candidate| {
        candidate.limit_window_seconds == window.limit_window_seconds
            && candidate.start_at == start_at
            && candidate.end_at == window.reset_at
    })
}

fn project_history_points(window: &UsageWindowHistory) -> Vec<ChartPoint> {
    let total = (window.end_at - window.start_at) as f64;
    if total <= 0.0 {
        return Vec::new();
    }

    let mut points = window
        .observations
        .iter()
        .map(|observation| ChartPoint {
            x: (((observation.observed_at - window.start_at) as f64 / total) * 7.0).clamp(0.0, 7.0),
            y: observation.used_percent.clamp(0.0, 100.0),
        })
        .collect::<Vec<_>>();
    points.sort_by(|left, right| left.x.total_cmp(&right.x));
    points.dedup_by(|left, right| (left.x - right.x).abs() < f64::EPSILON && (left.y - right.y).abs() < f64::EPSILON);
    points
}

fn pick_five_hour_window(usage: &UsageResponse) -> Option<&UsageWindow> {
    let rate_limit = usage.rate_limit.as_ref()?;
    if rate_limit
        .primary_window
        .as_ref()
        .is_some_and(|window| window.limit_window_seconds == 18_000)
    {
        return rate_limit.primary_window.as_ref();
    }
    if rate_limit
        .secondary_window
        .as_ref()
        .is_some_and(|window| window.limit_window_seconds == 18_000)
    {
        return rate_limit.secondary_window.as_ref();
    }
    None
}

fn pick_weekly_window(usage: &UsageResponse) -> Option<&UsageWindow> {
    let rate_limit = usage.rate_limit.as_ref()?;
    if rate_limit
        .secondary_window
        .as_ref()
        .is_some_and(|window| window.limit_window_seconds == 604_800)
    {
        return rate_limit.secondary_window.as_ref();
    }
    if rate_limit
        .primary_window
        .as_ref()
        .is_some_and(|window| window.limit_window_seconds == 604_800)
    {
        return rate_limit.primary_window.as_ref();
    }
    None
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_starts_on_current_profile_and_toggles_pane_focus() {
        let mut app = App::from_profile_names(vec!["Alpha".to_string(), "Beta".to_string()], 1);
        assert_eq!(app.pane_focus, PaneFocus::Accounts);
        assert_eq!(app.selected_profile_label(), Some("Beta"));

        app.pane_focus = app.pane_focus.toggle();
        assert_eq!(app.pane_focus, PaneFocus::Plot);
        assert_eq!(app.selected_profile_label(), Some("Beta"));
    }

    #[test]
    fn matching_window_history_projects_real_observation_points() {
        let history = UsageWindowHistory {
            limit_window_seconds: 604_800,
            start_at: 100,
            end_at: 604_900,
            observations: vec![
                crate::usage::UsageObservation {
                    observed_at: 100,
                    used_percent: 12.0,
                },
                crate::usage::UsageObservation {
                    observed_at: 302_500,
                    used_percent: 44.0,
                },
                crate::usage::UsageObservation {
                    observed_at: 604_900,
                    used_percent: 70.0,
                },
            ],
        };

        let points = project_history_points(&history);
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], ChartPoint { x: 0.0, y: 12.0 });
        assert!(points[1].x > 3.4 && points[1].x < 3.6);
        assert_eq!(points[2], ChartPoint { x: 7.0, y: 70.0 });
    }

    #[test]
    fn five_hour_subframe_is_bounded_inside_weekly_chart_space() {
        // weekly: 0..604800, currently at 60%
        // five_hour: 522000..540000 (start = reset_at - limit = 540000 - 18000), currently at 30%
        let weekly = UsageWindow {
            used_percent: 60.0,
            limit_window_seconds: 604_800,
            reset_after_seconds: 86_400,
            reset_at: 604_800,
        };
        let five_hour = UsageWindow {
            used_percent: 30.0,
            limit_window_seconds: 18_000,
            reset_after_seconds: 1_800,
            reset_at: 540_000,
        };

        // With history: first observation at or after five_hour_start (522000) is at 522000, used=45%
        // lower_y = 45.0 (data lookup)
        // 7d_delta = 60 - 45 = 15, 5h_used = 30
        // upper_y = 45 + (15/30)*100 = 45 + 50 = 95
        let history = crate::usage::UsageWindowHistory {
            limit_window_seconds: 604_800,
            start_at: 0,
            end_at: 604_800,
            observations: vec![
                crate::usage::UsageObservation { observed_at: 300_000, used_percent: 30.0 },
                crate::usage::UsageObservation { observed_at: 522_000, used_percent: 45.0 },
                crate::usage::UsageObservation { observed_at: 604_800, used_percent: 60.0 },
            ],
        };
        let subframe = build_five_hour_subframe(Some(&weekly), Some(&five_hour), Some(&history));
        assert!(subframe.available);
        assert!(subframe.start_x.unwrap() < subframe.end_x.unwrap());
        assert!(subframe.end_x.unwrap() <= 7.0);
        assert_eq!(subframe.lower_y, Some(45.0));
        assert_eq!(subframe.upper_y, Some(95.0));

        // Without history: fallback to weekly_used - 5h_used = 30, upper = 30 + (30/30)*100 = 130 → 100
        let subframe_no_hist = build_five_hour_subframe(Some(&weekly), Some(&five_hour), None);
        assert_eq!(subframe_no_hist.lower_y, Some(30.0));
        assert_eq!(subframe_no_hist.upper_y, Some(100.0));
    }
}

#[cfg(test)]
mod duration_tests {
    use super::*;

    #[test]
    fn format_duration_short_covers_all_tiers() {
        assert_eq!(format_duration_short(293927), "3d 9h");  // 3*86400 + 9*3600 + 38*60 + 47
        assert_eq!(format_duration_short(86400),  "1d");
        assert_eq!(format_duration_short(9000),   "2h 30m");
        assert_eq!(format_duration_short(3600),   "1h");
        assert_eq!(format_duration_short(120),    "2m");
        assert_eq!(format_duration_short(30),     "1m");     // < 1m rounds up to 1m
        assert_eq!(format_duration_short(0),      "1m");
        assert_eq!(format_duration_short(-1),     "1m");     // negative clamped
    }
}
