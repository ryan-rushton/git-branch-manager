use ratatui::{
  layout::Rect,
  style::{Color, Style},
  widgets::{Block, Borders, Paragraph},
};

use crate::{components::views::stash_list::stash_item::StashItem, tui::Frame};

#[derive(Default)]
pub struct InstructionFooter {}

impl InstructionFooter {
  pub fn render(&mut self, frame: &mut Frame<'_>, area: Rect, selected: Option<&StashItem>) {
    let instructions = if let Some(selected) = selected {
      if selected.staged_for_deletion {
        "D: Delete | Shift+D: Unstage | Ctrl+D: Delete All Staged | Tab: Switch to Branches"
      } else {
        "A: Apply | P: Pop | D: Stage for Deletion | Tab: Switch to Branches"
      }
    } else {
      "A: Apply | P: Pop | D: Stage for Deletion | Tab: Switch to Branches"
    };

    let paragraph = Paragraph::new(instructions)
      .block(Block::default().borders(Borders::ALL))
      .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
  }
}
