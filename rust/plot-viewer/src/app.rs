use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::prelude::*;
use ratatui::style::{Modifier, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use serde_json::Value;

use crate::input::{self, InputAction};
use crate::render;
use crate::render::{
    ChartPoint, ChartState, FiveHourBandState, FocusTarget, RenderProfile, SelectionState,
};
use crate::store::{AccountStore, SavedProfile};
use crate::usage::{UsageReadResult, UsageResponse, UsageService, UsageSource, UsageWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Accounts,
    Plot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusPanel {
    Chart,
    Summary,
}

impl FocusPanel {
    fn next(self) -> Self {
        match self {
            Self::Chart => Self::Summary,
            Self::Summary => Self::Chart,
        }
    }

    fn previous(self) -> Self {
        self.next()
    }

    fn as_target(self) -> FocusTarget {
        match self {
            Self::Chart => FocusTarget::Chart,
            Self::Summary => FocusTarget::Summary,
        }
    }
}

#[derive(Debug, Clone)]
struct ProfileEntry {
    saved_name: Option<String>,
    profile_name: String,
    snapshot: Value,
    usage_view: UsageReadResult,
    account_id: Option<String>,
    is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DialogMode {
    SaveCurrent,
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
    focus: FocusPanel,
    view_mode: ViewMode,
    should_quit: bool,
    dialog: Option<DialogState>,
    status_message: Option<String>,
    store: Option<AccountStore>,
    usage_service: Option<UsageService>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AppRenderState<'a> {
    profiles: &'a [ProfileEntry],
    selected_profile_index: usize,
    focus: FocusPanel,
}

impl App {
    pub fn load(store: AccountStore, usage_service: UsageService) -> Result<Self> {
        let profiles = load_profiles(&store, &usage_service, false, None)?;
        Ok(Self {
            selected_profile_index: initial_selected_index(&profiles),
            profiles,
            focus: FocusPanel::Chart,
            view_mode: ViewMode::Accounts,
            should_quit: false,
            dialog: None,
            status_message: None,
            store: Some(store),
            usage_service: Some(usage_service),
        })
    }

    pub fn from_profile_names(profile_names: Vec<String>, selected_profile_index: usize) -> Self {
        let profiles = profile_names
            .into_iter()
            .enumerate()
            .map(|(index, name)| ProfileEntry {
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
            })
            .collect::<Vec<_>>();

        Self {
            profiles,
            selected_profile_index,
            focus: FocusPanel::Chart,
            view_mode: ViewMode::Accounts,
            should_quit: false,
            dialog: None,
            status_message: None,
            store: None,
            usage_service: None,
        }
    }

    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    pub fn selected_profile_label(&self) -> Option<&str> {
        self.selected_profile().map(|profile| profile.profile_name.as_str())
    }

    pub fn toggle_plot_mode(mut self) -> Self {
        self.view_mode = self.next_view_mode();
        self
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
        }
        Ok(())
    }

    fn handle_action(&mut self, action: InputAction) -> Result<()> {
        if self.dialog.is_some() {
            return self.handle_dialog_action(action);
        }

        match action {
            InputAction::Quit => self.should_quit = true,
            InputAction::Up => self.step_profile(-1),
            InputAction::Down => self.step_profile(1),
            InputAction::Left => {
                if self.view_mode == ViewMode::Plot {
                    self.step_profile(-1);
                }
            }
            InputAction::Right => {
                if self.view_mode == ViewMode::Plot {
                    self.step_profile(1);
                }
            }
            InputAction::TogglePlot => self.view_mode = self.next_view_mode(),
            InputAction::NextFocus => {
                if self.view_mode == ViewMode::Plot {
                    self.focus = self.focus.next();
                }
            }
            InputAction::PreviousFocus => {
                if self.view_mode == ViewMode::Plot {
                    self.focus = self.focus.previous();
                }
            }
            InputAction::Enter => self.activate_selected_profile()?,
            InputAction::Rename => self.open_rename_dialog(),
            InputAction::Delete => self.open_delete_dialog(),
            InputAction::RefreshSelected => self.refresh_selected_profile(true)?,
            InputAction::RefreshAll => self.reload_profiles(true, None)?,
            InputAction::Backspace | InputAction::Character(_) | InputAction::Cancel => {}
        }

        Ok(())
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
            DialogMode::SaveCurrent => {
                let Some(store) = self.store.as_ref() else {
                    return Ok(());
                };
                let Some(profile) = self.selected_profile().cloned() else {
                    return Ok(());
                };
                let name = store
                    .save_snapshot(&dialog.input, &profile.snapshot)
                    .with_context(|| format!("save snapshot {}", dialog.input))?;
                self.status_message = Some(format!("Saved current profile as \"{name}\"."));
                self.dialog = None;
                self.reload_profiles(false, profile.account_id.clone())?;
            }
            DialogMode::RenameSaved(current_name) => {
                let Some(store) = self.store.as_ref() else {
                    return Ok(());
                };
                let name = store
                    .rename_account(&current_name, &dialog.input)
                    .with_context(|| format!("rename profile {current_name}"))?;
                self.status_message = Some(format!("Renamed to \"{name}\"."));
                self.dialog = None;
                self.reload_profiles(false, None)?;
            }
            DialogMode::ConfirmDelete(target_name) => {
                if dialog.input.trim().eq_ignore_ascii_case("y")
                    || dialog.input.trim().eq_ignore_ascii_case("yes")
                {
                    let Some(store) = self.store.as_ref() else {
                        return Ok(());
                    };
                    store
                        .delete_account(&target_name)
                        .with_context(|| format!("delete profile {target_name}"))?;
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

        if let Some(saved_name) = profile.saved_name.as_deref() {
            if let Some(store) = self.store.as_ref() {
                let activated = store.use_account(saved_name)?;
                self.status_message = Some(format!("Switched Codex auth to \"{activated}\"."));
                self.reload_profiles(false, profile.account_id.clone())?;
            }
            return Ok(());
        }

        self.dialog = Some(DialogState {
            mode: DialogMode::SaveCurrent,
            input: build_default_name(profile.usage_view.usage.as_ref(), &profile.snapshot),
        });
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
        let Some(store) = self.store.as_ref() else {
            return Ok(());
        };
        let Some(usage_service) = self.usage_service.as_ref() else {
            return Ok(());
        };
        self.profiles = load_profiles(store, usage_service, force_refresh, refresh_account_id.as_deref())?;
        self.selected_profile_index = self
            .selected_profile_index
            .min(self.profiles.len().saturating_sub(1));
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

    fn next_view_mode(&self) -> ViewMode {
        match self.view_mode {
            ViewMode::Accounts => ViewMode::Plot,
            ViewMode::Plot => ViewMode::Accounts,
        }
    }

    fn render(&self, frame: &mut Frame) {
        match self.view_mode {
            ViewMode::Accounts => self.render_accounts(frame),
            ViewMode::Plot => self.render_plot(frame),
        }

        if self.dialog.is_some() {
            self.render_dialog(frame);
        }
    }

    fn render_accounts(&self, frame: &mut Frame) {
        let outer = Block::default().title("codex-auth").borders(Borders::ALL);
        let inner = outer.inner(frame.area());
        frame.render_widget(outer, frame.area());
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let [list_area, detail_area, footer_area] = Layout::vertical([
            Constraint::Min(8),
            Constraint::Length(9),
            Constraint::Length(3),
        ])
        .areas(inner);

        let items = self
            .profiles
            .iter()
            .map(|profile| {
                let prefix = if profile.is_current { "▶" } else { " " };
                let saved = if profile.saved_name.is_some() {
                    "saved"
                } else {
                    "unsaved"
                };
                let usage = format_usage_badge(&profile.usage_view);
                ListItem::new(format!("{prefix} {} [{saved}] {usage}", profile.profile_name))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        state.select((!self.profiles.is_empty()).then_some(self.selected_profile_index));
        let list = List::new(items)
            .block(Block::default().title("Profiles").borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, list_area, &mut state);

        let detail_lines = render_account_detail(self.selected_profile(), self.status_message.as_deref());
        let details = Paragraph::new(Text::from(detail_lines))
            .block(Block::default().title("Details").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(details, detail_area);

        let footer = Paragraph::new(Text::from(vec![
            Line::from("Enter=switch/save current · n=rename · d=delete · u=refresh one · a=refresh all"),
            Line::from("p/b=plot view · q=quit"),
        ]))
        .block(Block::default().title("Actions").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
        frame.render_widget(footer, footer_area);
    }

    fn render_plot(&self, frame: &mut Frame) {
        let render_state = AppRenderState {
            profiles: &self.profiles,
            selected_profile_index: self.selected_profile_index,
            focus: self.focus,
        };
        render::render(frame, frame.area(), &render_state);
    }

    fn render_dialog(&self, frame: &mut Frame) {
        let Some(dialog) = self.dialog.as_ref() else {
            return;
        };
        let title = match &dialog.mode {
            DialogMode::SaveCurrent => "Save current profile",
            DialogMode::RenameSaved(_) => "Rename saved profile",
            DialogMode::ConfirmDelete(_) => "Delete saved profile",
        };
        let prompt = match &dialog.mode {
            DialogMode::SaveCurrent => "Enter a name for the current auth snapshot.",
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
            focus: self.focus.as_target(),
        }
    }

    fn chart_state(&self) -> ChartState<'_> {
        let Some(profile) = self.selected_profile() else {
            return empty_chart_state("no selected profile");
        };
        build_chart_state(profile.usage_view.usage.as_ref())
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
            profiles.push(ProfileEntry {
                saved_name: None,
                profile_name: format!(
                    "{} [UNSAVED]",
                    build_default_name(usage_view.usage.as_ref(), &snapshot)
                ),
                account_id: current_account_id.clone(),
                is_current: true,
                snapshot,
                usage_view,
            });
        }
    }

    profiles.sort_by(|left, right| left.profile_name.cmp(&right.profile_name));
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

    Ok(ProfileEntry {
        saved_name: Some(profile.name.clone()),
        profile_name: profile.name,
        snapshot: profile.snapshot,
        usage_view,
        account_id: account_id.clone(),
        is_current: current_account_id.as_deref() == account_id.as_deref(),
    })
}

fn initial_selected_index(profiles: &[ProfileEntry]) -> usize {
    profiles.iter().position(|profile| profile.is_current).unwrap_or(0)
}

fn render_account_detail(
    profile: Option<&ProfileEntry>,
    status_message: Option<&str>,
) -> Vec<Line<'static>> {
    let Some(profile) = profile else {
        return vec![
            Line::from("No Codex auth profile loaded."),
            Line::from("Save or switch an account to continue."),
        ];
    };

    let mut lines = vec![
        Line::from(format!("Profile: {}", profile.profile_name)),
        Line::from(format!(
            "State: {}{}",
            if profile.is_current { "current" } else { "saved" },
            if profile.saved_name.is_none() { " · unsaved" } else { "" }
        )),
        Line::from(format!(
            "Account: {}",
            profile.account_id.as_deref().unwrap_or("n/a")
        )),
        Line::from(format!(
            "Usage source: {}{}",
            match profile.usage_view.source {
                UsageSource::Api => "api",
                UsageSource::Cache => "cache",
                UsageSource::None => "none",
            },
            if profile.usage_view.stale { " (stale)" } else { "" }
        )),
    ];

    if let Some(usage) = profile.usage_view.usage.as_ref() {
        lines.push(Line::from(format!(
            "Email / plan: {} / {}",
            usage.email.as_deref().unwrap_or("unknown"),
            usage.plan_type.as_deref().unwrap_or("unknown")
        )));
        lines.push(Line::from(format!(
            "Weekly / 5h: {} / {}",
            summarize_window(pick_weekly_window(usage)),
            summarize_window(pick_five_hour_window(usage))
        )));
    }

    if let Some(message) = status_message {
        lines.push(Line::from(""));
        lines.push(Line::from(message.to_string()));
    }

    lines
}

fn summarize_window(window: Option<&UsageWindow>) -> String {
    let Some(window) = window else {
        return "n/a".to_string();
    };
    format!(
        "{:.0}% used, reset in {}s",
        window.used_percent, window.reset_after_seconds
    )
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

fn build_chart_state(usage: Option<&UsageResponse>) -> ChartState<'static> {
    let Some(usage) = usage else {
        return empty_chart_state("no usage data");
    };
    let weekly = pick_weekly_window(usage);
    let five_hour = pick_five_hour_window(usage);

    let seven_day_points = weekly
        .map(build_weekly_points)
        .unwrap_or_else(|| vec![ChartPoint { x: 0.0, y: 0.0 }]);
    let five_hour_band = if let Some(window) = five_hour {
        let used = window.used_percent.clamp(0.0, 100.0);
        let lower = (used - 10.0).max(0.0);
        let upper = (used + 10.0).min(100.0);
        FiveHourBandState {
            available: true,
            lower_y: Some(lower),
            upper_y: Some(upper),
            delta_seven_day_percent: weekly.map(|weekly| used - weekly.used_percent),
            delta_five_hour_percent: Some(0.0),
            reason: None,
        }
    } else {
        FiveHourBandState {
            available: false,
            lower_y: None,
            upper_y: None,
            delta_seven_day_percent: None,
            delta_five_hour_percent: None,
            reason: Some("no 5h window"),
        }
    };

    ChartState {
        seven_day_points,
        five_hour_band,
    }
}

fn empty_chart_state(reason: &'static str) -> ChartState<'static> {
    ChartState {
        seven_day_points: Vec::new(),
        five_hour_band: FiveHourBandState {
            available: false,
            lower_y: None,
            upper_y: None,
            delta_seven_day_percent: None,
            delta_five_hour_percent: None,
            reason: Some(reason),
        },
    }
}

fn build_weekly_points(window: &UsageWindow) -> Vec<ChartPoint> {
    let total = (window.limit_window_seconds as f64).max(1.0);
    let elapsed = (total - (window.reset_after_seconds as f64).max(0.0)).clamp(0.0, total);
    let progress_days = (elapsed / total) * 7.0;
    let used = window.used_percent.clamp(0.0, 100.0);

    vec![
        ChartPoint { x: 0.0, y: 0.0 },
        ChartPoint {
            x: (progress_days / 2.0).clamp(0.0, 7.0),
            y: (used / 2.0).clamp(0.0, 100.0),
        },
        ChartPoint {
            x: progress_days.clamp(0.0, 7.0),
            y: used,
        },
        ChartPoint { x: 7.0, y: used },
    ]
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
    fn app_starts_on_current_profile_and_toggles_plot_mode() {
        let app = App::from_profile_names(vec!["Alpha".to_string(), "Beta".to_string()], 1);
        assert_eq!(app.view_mode(), ViewMode::Accounts);
        assert_eq!(app.selected_profile_label(), Some("Beta"));

        let app = app.toggle_plot_mode();
        assert_eq!(app.view_mode(), ViewMode::Plot);
        assert_eq!(app.selected_profile_label(), Some("Beta"));
    }

    #[test]
    fn weekly_window_becomes_simple_visible_line_series() {
        let usage = UsageResponse {
            email: Some("a@example.com".to_string()),
            plan_type: Some("plus".to_string()),
            rate_limit: Some(crate::usage::UsageRateLimit {
                primary_window: None,
                secondary_window: Some(UsageWindow {
                    used_percent: 42.0,
                    limit_window_seconds: 604_800,
                    reset_after_seconds: 302_400,
                    reset_at: 123,
                }),
            }),
        };

        let chart = build_chart_state(Some(&usage));
        assert_eq!(chart.seven_day_points.len(), 4);
        assert!(chart.seven_day_points.iter().any(|point| point.y > 0.0));
    }
}
