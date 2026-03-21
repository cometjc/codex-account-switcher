use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::Frame;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::{RenderContext, RenderState};

/// Visible chart scaffold for the planned plot viewer.
///
/// This keeps the render boundary stable while still showing a meaningful
/// summary block that future Canvas-based charting can replace.
pub fn render_chart<State: RenderState>(frame: &mut Frame, context: &RenderContext<'_, State>) {
    let block = Block::default()
        .title("plot chart")
        .borders(Borders::ALL);
    let inner = block.inner(context.area);
    frame.render_widget(block, context.area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(2),
    ])
    .split(inner);

    let overview = Paragraph::new(Text::from(vec![
        Line::from(format!("Selected: {}", context.state.selected_profile_label())),
        Line::from(format!("Current: {}", context.state.snapshot_active_label())),
        Line::from(format!("Focus: {}", context.state.focus_label())),
        Line::from("7d curve: pending Canvas plot"),
        Line::from("5h band: pending geometry bind"),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(overview, chunks[0]);

    let scaffold = Paragraph::new(Text::from(vec![
        Line::from("7d axis: 0% -------------------- 100%"),
        Line::from("5h band: [reserved for projected overlap lines]"),
        Line::from("Chart scaffold is visible and ready for the real plot renderer."),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(scaffold, chunks[1]);
}
