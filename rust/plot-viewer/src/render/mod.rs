pub mod chart;
pub mod panels;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Shared render inputs for the Rust codex-auth plot view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderContext<'a, State> {
    pub state: &'a State,
    pub area: Rect,
    pub viewport: RenderViewport,
}

impl<'a, State> RenderContext<'a, State> {
    pub fn new(state: &'a State, area: Rect) -> Self {
        Self {
            state,
            area,
            viewport: RenderViewport::from(area),
        }
    }

    pub fn with_area(&self, area: Rect) -> Self {
        Self {
            state: self.state,
            area,
            viewport: RenderViewport::from(area),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderViewport {
    pub width: u16,
    pub height: u16,
}

impl From<Rect> for RenderViewport {
    fn from(area: Rect) -> Self {
        Self {
            width: area.width,
            height: area.height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    Chart,
    Summary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderProfile<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub is_current: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionState<'a> {
    pub selected: Option<RenderProfile<'a>>,
    pub current: Option<RenderProfile<'a>>,
    pub focus: FocusTarget,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChartSeriesStyle {
    pub color_slot: usize,
    pub is_selected: bool,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChartSeries<'a> {
    pub profile: RenderProfile<'a>,
    pub style: ChartSeriesStyle,
    pub points: Vec<ChartPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiveHourBandState<'a> {
    pub available: bool,
    pub lower_y: Option<f64>,
    pub upper_y: Option<f64>,
    pub delta_seven_day_percent: Option<f64>,
    pub delta_five_hour_percent: Option<f64>,
    pub reason: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiveHourSubframeState<'a> {
    pub available: bool,
    pub start_x: Option<f64>,
    pub end_x: Option<f64>,
    pub lower_y: Option<f64>,
    pub upper_y: Option<f64>,
    pub reason: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChartState<'a> {
    pub series: Vec<ChartSeries<'a>>,
    pub seven_day_points: Vec<ChartPoint>,
    pub five_hour_band: FiveHourBandState<'a>,
    pub five_hour_subframe: FiveHourSubframeState<'a>,
    pub total_points: usize,
}

pub trait RenderState {
    fn selection_state(&self) -> SelectionState<'_>;
    fn chart_state(&self) -> ChartState<'_>;

    fn selected_profile_label(&self) -> &str {
        self.selection_state()
            .selected
            .map(|profile| profile.label)
            .unwrap_or("no profiles loaded")
    }
}

/// Entry point for the codex-auth plot layout boundary.
pub fn render<State: RenderState>(frame: &mut Frame, area: Rect, state: &State) {
    let context = RenderContext::new(state, area);
    let outer = Block::default().title("codex-auth plot").borders(Borders::ALL);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .split(inner);

    let header = Paragraph::new(Text::from(vec![
        Line::from("Rust codex-auth plot view"),
        Line::from(format!(
            "Selected profile: {}",
            state.selected_profile_label()
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(header, chunks[0]);

    chart::render_chart(frame, &context.with_area(chunks[1]));

    let footer = Paragraph::new(Text::from(vec![Line::from(format!(
        "Viewport: {}x{} · Tab switches focus · Left/Right cycles profiles",
        context.viewport.width, context.viewport.height
    ))]))
    .wrap(Wrap { trim: true });
    frame.render_widget(footer, chunks[2]);

    panels::render_panels(&context.with_area(chunks[2]));
}
