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
    let mut instructions = vec!["esc: Exit"];
    if let Some(selected) = selected {
      if selected.staged_for_deletion {
        instructions.push("d: Delete");
        instructions.push("shift+d: Unstage");
        instructions.push("ctrl+d: Delete All Staged");
      } else {
        instructions.push("c: Checkout");
        instructions.push("shift+c: Create New");
        instructions.push("d: Stage for Deletion");
      }
    } else {
      instructions.push("c: Checkout");
      instructions.push("shift+c: Create New");
      instructions.push("d: Stage for Deletion");
    };

    instructions.push("tab: Switch to Stashes");

    let paragraph = Paragraph::new(instructions.join(" | "))
      .block(Block::default().borders(Borders::ALL))
      .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
  }
}
