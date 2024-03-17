use ratatui::{
  layout::Rect,
  prelude::{Line, Span},
};

use crate::{components::branch_list::branch_item::BranchItem, tui::Frame};

#[derive(Debug, Default)]
pub struct InstructionFooter {}

impl InstructionFooter {
  pub fn render(&self, f: &mut Frame<'_>, area: Rect, branches: &[BranchItem], selected: Option<&BranchItem>) {
    let mut commands = vec![Span::raw("q: Quit")];
    commands.push(Span::raw(" | ⇧ + c: Checkout new"));
    if selected.is_some() && selected.unwrap().staged_for_deletion {
      commands.push(Span::raw(" | d: Delete"));
      commands.push(Span::raw(" | ⇧ + d: Unstage for deletion"));
    }

    if selected.is_some() && !selected.unwrap().branch.is_head {
      commands.push(Span::raw(" | d: Stage for deletion"));
    }

    if selected.is_some() {
      commands.push(Span::raw(" | c: Checkout"));
    }

    if branches.iter().any(|b| b.staged_for_deletion) {
      commands.push(Span::raw(" | ^ + d: Delete all staged branches"));
    }

    let footer = Line::from(commands);
    f.render_widget(footer, area);
  }
}
