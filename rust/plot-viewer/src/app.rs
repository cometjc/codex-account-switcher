use std::time::Duration;

use anyhow::Result;
use crossterm::event;
use ratatui::prelude::*;

use crate::input::{self, InputAction};
use crate::model::{PlotProfile, PlotSnapshot};
use crate::render;
use crate::render::{
    ChartPoint, ChartState, FiveHourBandState, FocusTarget, RenderProfile, SelectionState,
};

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
    fn selection_state(&self) -> SelectionState<'_> {
        SelectionState {
            selected: self.selected_profile().map(RenderProfile::from),
            current: self.snapshot.current_profile().map(RenderProfile::from),
            focus: self.focus.as_target(),
        }
    }

    fn chart_state(&self) -> ChartState<'_> {
        let Some(profile) = self.selected_profile() else {
            return ChartState {
                seven_day_points: Vec::new(),
                five_hour_band: FiveHourBandState {
                    available: false,
                    lower_y: None,
                    upper_y: None,
                    delta_seven_day_percent: None,
                    delta_five_hour_percent: None,
                    reason: Some("no selected profile"),
                },
            };
        };

        ChartState {
            seven_day_points: profile
                .seven_day_points
                .iter()
                .map(|point| ChartPoint {
                    x: point.offset_seconds as f64 / 86_400.0,
                    y: point.used_percent,
                })
                .collect(),
            five_hour_band: FiveHourBandState {
                available: profile.five_hour_band.available,
                lower_y: profile.five_hour_band.lower_y,
                upper_y: profile.five_hour_band.upper_y,
                delta_seven_day_percent: profile.five_hour_band.delta_seven_day_percent,
                delta_five_hour_percent: profile.five_hour_band.delta_five_hour_percent,
                reason: profile.five_hour_band.reason.as_deref(),
            },
        }
    }
}

impl<'a> AppRenderState<'a> {
    fn selected_profile(&self) -> Option<&'a PlotProfile> {
        self.snapshot.profiles.get(self.selected_profile_index)
    }
}

impl<'a> From<&'a PlotProfile> for RenderProfile<'a> {
    fn from(profile: &'a PlotProfile) -> Self {
        Self {
            id: profile.id.as_str(),
            label: profile.name.as_str(),
            is_current: profile.is_current,
        }
    }
}

impl FocusPanel {
    fn as_target(self) -> FocusTarget {
        match self {
            Self::Chart => FocusTarget::Chart,
            Self::Summary => FocusTarget::Summary,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::RenderState;

    fn sample_snapshot() -> PlotSnapshot {
        let mut snapshot = PlotSnapshot {
            schema_version: 1,
            generated_at: 1_742_611_200,
            current_profile_id: Some("beta".to_string()),
            profiles: vec![
                PlotProfile {
                    id: "alpha".to_string(),
                    name: "Alpha".to_string(),
                    is_current: false,
                    usage: None,
                    seven_day_window: crate::model::PlotWindowBounds {
                        start_at: None,
                        end_at: None,
                    },
                    seven_day_points: Vec::new(),
                    five_hour_window: crate::model::PlotWindowBounds {
                        start_at: None,
                        end_at: None,
                    },
                    five_hour_band: crate::model::PlotFiveHourBand {
                        available: false,
                        lower_y: None,
                        upper_y: None,
                        band_height: None,
                        delta_seven_day_percent: None,
                        delta_five_hour_percent: None,
                        reason: None,
                    },
                    summary_labels: crate::model::PlotSummaryLabels::default(),
                },
                PlotProfile {
                    id: "beta".to_string(),
                    name: "Beta".to_string(),
                    is_current: true,
                    usage: None,
                    seven_day_window: crate::model::PlotWindowBounds {
                        start_at: None,
                        end_at: None,
                    },
                    seven_day_points: Vec::new(),
                    five_hour_window: crate::model::PlotWindowBounds {
                        start_at: None,
                        end_at: None,
                    },
                    five_hour_band: crate::model::PlotFiveHourBand {
                        available: true,
                        lower_y: Some(10.0),
                        upper_y: Some(20.0),
                        band_height: Some(10.0),
                        delta_seven_day_percent: Some(1.0),
                        delta_five_hour_percent: Some(2.0),
                        reason: None,
                    },
                    summary_labels: crate::model::PlotSummaryLabels::default(),
                },
            ],
            active_profile_index: 0,
        };
        snapshot.active_profile_index = snapshot.current_profile_index().unwrap_or(0);
        snapshot
    }

    #[test]
    fn render_state_exposes_current_selected_and_focus_as_one_contract() {
        let app = App::new(sample_snapshot());
        let render_state = AppRenderState {
            snapshot: &app.snapshot,
            selected_profile_index: app.selected_profile_index,
            focus: app.focus,
        };

        let selection = render_state.selection_state();

        assert_eq!(
            selection.selected,
            Some(RenderProfile {
                id: "beta",
                label: "Beta",
                is_current: true,
            })
        );
        assert_eq!(selection.current, selection.selected);
        assert_eq!(selection.focus, FocusTarget::Chart);
    }

    #[test]
    fn render_state_tracks_profile_and_focus_changes_together() {
        let mut app = App::new(sample_snapshot());

        app.handle_action(InputAction::PreviousProfile);
        app.handle_action(InputAction::NextFocus);

        let render_state = AppRenderState {
            snapshot: &app.snapshot,
            selected_profile_index: app.selected_profile_index,
            focus: app.focus,
        };

        let selection = render_state.selection_state();

        assert_eq!(
            selection.selected,
            Some(RenderProfile {
                id: "alpha",
                label: "Alpha",
                is_current: false,
            })
        );
        assert_eq!(
            selection.current,
            Some(RenderProfile {
                id: "beta",
                label: "Beta",
                is_current: true,
            })
        );
        assert_eq!(selection.focus, FocusTarget::Summary);
    }
}
