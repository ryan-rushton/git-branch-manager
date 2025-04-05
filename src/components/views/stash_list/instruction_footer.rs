use ratatui::{
  layout::Rect,
  style::{Color, Style},
  widgets::{Block, Borders, Paragraph},
};

use crate::{components::views::stash_list::stash_item::StashItem, tui::Frame};

#[derive(Default)]
pub struct InstructionFooter {}

impl InstructionFooter {
  pub fn render(
    &mut self,
    frame: &mut Frame<'_>,
    area: Rect,
    selected: Option<&StashItem>,
    has_staged_for_deletion: bool,
  ) {
    let mut instructions = vec!["esc: Exit", "s: New Stash"];

    // Assume something is selected means we have stashes to work with
    if let Some(selected) = selected {
      instructions.push("a: Apply");
      instructions.push("p: Pop");

      if selected.staged_for_deletion {
        instructions.push("d: Delete");
        instructions.push("shift+d: Unstage");
      } else {
        instructions.push("d: Stage for Deletion");
      }
    }

    if has_staged_for_deletion {
      instructions.push("ctrl+d: Delete All Staged");
    }

    instructions.push("tab: Switch to Branches"); // Always add Tab

    let paragraph = Paragraph::new(instructions.join(" | "))
      .block(Block::default().borders(Borders::ALL))
      .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
  }
}
