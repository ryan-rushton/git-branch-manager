use ratatui::{
  style::{Color, Style},
  text::{Line, Span},
  widgets::ListItem,
};

use crate::git::types::GitStash;

#[derive(Clone)]
pub struct StashItem {
  pub stash: GitStash,
  pub staged_for_deletion: bool,
}

impl StashItem {
  pub fn new(stash: GitStash) -> Self {
    StashItem { stash, staged_for_deletion: false }
  }

  pub fn stage_for_deletion(&mut self, stage: bool) {
    self.staged_for_deletion = stage;
  }

  pub fn render(&self) -> ListItem<'_> {
    let mut text = Line::default();
    let mut parts = Vec::new();

    let index = Span::styled(format!("{}: ", self.stash.index), Style::default());
    let mut message = Span::styled(self.stash.message.clone(), Style::default());

    if self.staged_for_deletion {
      message = message.style(Style::default().fg(Color::Red));
    }

    parts.push(index);
    parts.push(message);
    text = text.spans(parts);
    ListItem::from(text)
  }
}
