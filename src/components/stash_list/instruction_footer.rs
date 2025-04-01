use ratatui::{
  layout::Rect,
  prelude::{Line, Span},
};

use crate::{components::stash_list::stash_item::StashItem, tui::Frame};

#[derive(Debug, Default)]
pub struct InstructionFooter {}

impl InstructionFooter {
  pub fn render(&self, f: &mut Frame<'_>, area: Rect, stashes: &[StashItem], selected: Option<&StashItem>) {
    let mut commands = vec![Span::raw("esc: Quit")];
    let staged_for_deletion = selected.is_some() && selected.unwrap().staged_for_deletion;

    if selected.is_some() {
      if staged_for_deletion {
        commands.push(Span::raw(" | d: Drop"));
        commands.push(Span::raw(" | ⇧ + d: Unstage for deletion"));
      } else {
        commands.push(Span::raw(" | a: Apply"));
        commands.push(Span::raw(" | p: Pop"));
        commands.push(Span::raw(" | d: Stage for deletion"));
      }
    }

    if stashes.iter().any(|s| s.staged_for_deletion) {
      commands.push(Span::raw(" | ^ + d: Delete all staged stashes"));
    }

    let footer = Line::from(commands);
    f.render_widget(footer, area);
  }
}
