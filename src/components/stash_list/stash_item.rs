use ratatui::{
  style::{Color, Modifier, Style},
  text::{Line, Span},
  widgets::ListItem,
};

use crate::git::types::GitStash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashItem {
  pub git_stash: GitStash,
  pub staged_for_deletion: bool,
}

impl StashItem {
  pub fn new(git_stash: GitStash) -> Self {
    StashItem { git_stash, staged_for_deletion: false }
  }

  pub fn render(&self) -> ListItem {
    let mut text = Line::default();
    let mut parts = Vec::new();
    let mut index = Span::styled(self.git_stash.index.to_string(), Style::default());
    if self.staged_for_deletion {
      index = index.style(Style::default().fg(Color::Red));
    }
    parts.push(index);

    let message =
      Span::styled(format!(" {}", self.git_stash.message.clone()), Style::default().add_modifier(Modifier::DIM));
    parts.push(message);

    let id =
      Span::styled(format!(" ({})", self.git_stash.stash_id.clone()), Style::default().add_modifier(Modifier::DIM));
    parts.push(id);

    text = text.spans(parts);
    ListItem::from(text)
  }

  pub fn stage_for_deletion(&mut self, stage: bool) {
    self.staged_for_deletion = stage;
  }
}
