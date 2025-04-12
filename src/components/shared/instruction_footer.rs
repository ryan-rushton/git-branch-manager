use ratatui::{
  layout::Rect,
  style::{Color, Style},
  widgets::{Block, Borders, Paragraph},
};

use crate::tui::Frame; // Keep Frame import

#[derive(Default)]
pub struct InstructionFooter {}

impl InstructionFooter {
  /// Renders the footer with the provided instructions.
  pub fn render(&self, frame: &mut Frame<'_>, area: Rect, instructions: Vec<&'static str>) {
    // Instructions are now passed in directly

    if instructions.is_empty() {
      // Optionally render nothing or a default message if no instructions are provided
      return;
    }

    let text = instructions.join(" | ");
    let paragraph =
      Paragraph::new(text).block(Block::default().borders(Borders::ALL)).style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
  }
}
