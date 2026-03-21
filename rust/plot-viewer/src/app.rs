use std::time::Duration;

use anyhow::Result;
use crossterm::event;
use ratatui::prelude::*;

use crate::input::{self, InputAction};
use crate::model::{PlotProfile, PlotSnapshot};
use crate::render;

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
}

pub fn run(snapshot: PlotSnapshot) -> Result<()> {
    let mut terminal = TerminalSession::enter();
    let mut app = App::new(snapshot);
    terminal.run(&mut app)
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
        app.run(&mut self.terminal)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

struct App {
    snapshot: PlotSnapshot,
    selected_profile_index: usize,
    focus: FocusPanel,
    should_quit: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AppRenderState<'a> {
    snapshot: &'a PlotSnapshot,
    selected_profile_index: usize,
    focus: FocusPanel,
}

impl render::RenderState for AppRenderState<'_> {
    fn selected_profile_label(&self) -> &str {
        self.selected_profile()
            .map(|profile| profile.name.as_str())
            .unwrap_or("no profiles loaded")
    }

    fn snapshot_active_label(&self) -> &str {
        self.snapshot
            .active_profile()
            .map(|profile| profile.name.as_str())
            .unwrap_or("none")
    }

    fn focus_label(&self) -> &str {
        self.focus.as_label()
    }
}

impl<'a> AppRenderState<'a> {
    fn selected_profile(&self) -> Option<&'a PlotProfile> {
        self.snapshot.profiles.get(self.selected_profile_index)
    }
}

impl FocusPanel {
    fn as_label(self) -> &'static str {
        match self {
            Self::Chart => "Chart",
            Self::Summary => "Summary",
        }
    }
}

impl App {
    fn new(snapshot: PlotSnapshot) -> Self {
        let selected_profile_index = snapshot.active_profile_index.min(
            snapshot
                .profiles
                .len()
                .saturating_sub(1),
        );

        Self {
            snapshot,
            selected_profile_index,
            focus: FocusPanel::Chart,
            should_quit: false,
        }
    }

    fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;

            if event::poll(Duration::from_millis(150))? {
                let event = event::read()?;
                if let Some(action) = input::map_event(&event) {
                    self.handle_action(action);
                }
            }
        }

        Ok(())
    }

    fn handle_action(&mut self, action: InputAction) {
        match action {
            InputAction::Quit => self.should_quit = true,
            InputAction::NextProfile => self.step_profile(1),
            InputAction::PreviousProfile => self.step_profile(-1),
            InputAction::NextFocus => self.focus = self.focus.next(),
            InputAction::PreviousFocus => self.focus = self.focus.previous(),
        }
    }

    fn step_profile(&mut self, delta: isize) {
        let len = self.snapshot.profiles.len();
        if len == 0 {
            self.selected_profile_index = 0;
            return;
        }

        let len = len as isize;
        let current = self.selected_profile_index as isize;
        let next = (current + delta).rem_euclid(len);
        self.selected_profile_index = next as usize;
    }

    fn render(&self, frame: &mut Frame) {
        let render_state = AppRenderState {
            snapshot: &self.snapshot,
            selected_profile_index: self.selected_profile_index,
            focus: self.focus,
        };

        render::render(frame, frame.area(), &render_state);
    }
}
