pub mod chart;

use ratatui::layout::Rect;
use ratatui::prelude::{Color, Frame};

pub const SERIES_COLORS: [Color; 8] = [
    Color::Cyan,
    Color::Yellow,
    Color::Magenta,
    Color::Green,
    Color::LightBlue,
    Color::LightRed,
    Color::LightGreen,
    Color::White,
];

/// Shared render inputs for the Rust agent-switch plot view.
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
pub struct RenderProfile<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub is_current: bool,
    pub agent_type: &'a str,
    pub window_label: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionState<'a> {
    pub selected: Option<RenderProfile<'a>>,
    pub current: Option<RenderProfile<'a>>,
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
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChartSeries<'a> {
    pub profile: RenderProfile<'a>,
    pub style: ChartSeriesStyle,
    pub points: Vec<ChartPoint>,
    pub last_seven_day_percent: Option<f64>,
    pub five_hour_used_percent: Option<f64>,
    pub five_hour_subframe: FiveHourSubframeState<'a>,
    pub is_zero_state: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiveHourBandState<'a> {
    pub available: bool,
    pub used_percent: Option<f64>,
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
    pub y_lower: f64,
    pub y_upper: f64,
    pub x_lower: f64,          // X-axis left bound (days ago from origin)
    pub x_upper: f64,          // X-axis right bound: 7.0=now, less when panned into past
    pub solo: bool,             // if true, only render selected series
    pub tab_zoom_label: Option<&'a str>, // Some(name) = tab-zoomed to this profile
    pub focused: bool,          // true when Plot pane has keyboard focus
    pub fullscreen: bool,       // true when fullscreen mode is active
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

/// Entry point for the agent-switch plot layout boundary.
pub fn render<State: RenderState>(frame: &mut Frame, area: Rect, state: &State) {
    let context = RenderContext::new(state, area);
    if area.width == 0 || area.height == 0 {
        return;
    }
    chart::render_chart(frame, &context.with_area(area));
}
