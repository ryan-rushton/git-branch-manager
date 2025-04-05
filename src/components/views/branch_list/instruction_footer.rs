use ratatui::{
  layout::Rect,
  style::{Color, Style},
  widgets::{Block, Borders, Paragraph},
};

use crate::{components::views::branch_list::branch_item::BranchItem, tui::Frame};

#[derive(Default)]
pub struct InstructionFooter {}

impl InstructionFooter {
  pub fn render(
    &self,
    frame: &mut Frame<'_>,
    area: Rect,
    selected: Option<&BranchItem>,
    has_staged_for_deletion: bool,
  ) {
    let mut instructions = vec!["esc: Exit", "shift+c: Create New"];
    if let Some(selected) = selected {
      if selected.staged_for_deletion {
        instructions.push("d: Delete");
        instructions.push("shift+d: Unstage");
      } else {
        instructions.push("c: Checkout");
        instructions.push("d: Stage for Deletion");
      }
    }

    if has_staged_for_deletion {
      instructions.push("ctrl+d: Delete All Staged");
    }

    instructions.push("tab: Switch to Stashes");

    let paragraph = Paragraph::new(instructions.join(" | "))
      .block(Block::default().borders(Borders::ALL))
      .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
  }
}
