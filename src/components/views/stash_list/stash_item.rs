use ratatui::{
  style::{Color, Style},
  text::{Line, Span},
  widgets::ListItem,
};

use crate::{components::traits::list_item_wrapper::ListItemWrapper, git::types::GitStash};

#[derive(Clone, Debug)]
pub struct StashItem {
  pub stash: GitStash,
  pub staged_for_deletion: bool,
}

// impl StashItem { // Original impl block removed
// } // Original impl block removed

impl ListItemWrapper<GitStash> for StashItem {
  fn new(stash: GitStash) -> Self {
    StashItem { stash, staged_for_deletion: false }
  }

  fn stage_for_deletion(&mut self, stage: bool) {
    self.staged_for_deletion = stage;
  }

  fn is_staged_for_deletion(&self) -> bool {
    self.staged_for_deletion
  }

  fn inner_item(&self) -> &GitStash {
    &self.stash
  }

  fn render(&self) -> ListItem {
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
