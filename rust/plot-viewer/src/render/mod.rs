pub mod chart;
pub mod panels;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Frame;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Minimal shared render inputs for the plot viewer.
///
/// The future shell can extend this with richer layout hints without forcing
/// the render module to depend on app-specific state shapes.
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

pub trait RenderState {
    fn selected_profile_label(&self) -> &str;
    fn snapshot_active_label(&self) -> &str;
    fn focus_label(&self) -> &str;
}

/// Entry point for the plot-viewer layout boundary.
pub fn render<State: RenderState>(frame: &mut Frame, area: Rect, state: &State) {
    let context = RenderContext::new(state, area);
    let outer = Block::default().title("plot-viewer").borders(Borders::ALL);
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
        Line::from("Plot viewer shell"),
        Line::from(format!(
            "Selected profile: {}",
            state.selected_profile_label()
        )),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(header, chunks[0]);

    chart::render_chart(frame, &context.with_area(chunks[1]));

    let footer = Paragraph::new(Text::from(vec![Line::from(format!(
        "Viewport: {}x{}",
        context.viewport.width, context.viewport.height
    ))]))
    .wrap(Wrap { trim: true });
    frame.render_widget(footer, chunks[2]);

    panels::render_panels(&context.with_area(chunks[2]));
}
