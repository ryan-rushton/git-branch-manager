use ratatui::{
  layout::Rect,
  style::{Color, Style},
  widgets::{Block, Borders, Paragraph},
};

use crate::{components::views::branch_list::branch_item::BranchItem, tui::Frame};

#[derive(Default)]
pub struct InstructionFooter {}

impl InstructionFooter {
  pub fn render(&self, frame: &mut Frame<'_>, area: Rect, selected: Option<&BranchItem>) {
    let instructions = if let Some(selected) = selected {
      if selected.staged_for_deletion {
        "D: Delete | Shift+D: Unstage | Ctrl+D: Delete All Staged"
      } else {
        "C: Checkout | Shift+C: Create New | D: Stage for Deletion"
      }
    } else {
      "C: Checkout | Shift+C: Create New | D: Stage for Deletion"
    };

    let paragraph = Paragraph::new(instructions)
      .block(Block::default().borders(Borders::ALL))
      .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
  }
}
