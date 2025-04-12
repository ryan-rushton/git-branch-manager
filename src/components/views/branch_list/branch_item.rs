use ratatui::{
  style::{Color, Modifier, Style},
  text::{Line, Span},
  widgets::ListItem,
};

use crate::{components::traits::list_item_wrapper::ListItemWrapper, git::types::GitBranch};

#[derive(Debug, Clone)]
pub struct BranchItem {
  pub branch: GitBranch,
  pub staged_for_deletion: bool,
}

impl BranchItem {
  // Original new method, will be replaced by trait impl
  // pub fn new(branch: GitBranch) -> Self {
  //   BranchItem { branch, staged_for_deletion: false }
  // }

  // Original stage_for_deletion, will be moved to trait impl
  // pub fn stage_for_deletion(&mut self, stage: bool) {
  //   self.staged_for_deletion = stage;
  // }

  // Original render method, will be moved/adapted for trait impl
  // pub fn render(&self) -> ListItem {
  //   let mut text = Line::default();
  //   let mut parts = Vec::new();
  //   let mut name = Span::styled(self.branch.name.clone(), Style::default());
  //
  //   if self.staged_for_deletion {
  //     name = name.style(Style::default().fg(Color::Red));
  //   }
  //   // Removed creation styling
  //   parts.push(name);
  //
  //   if self.branch.is_head {
  //     parts.push(Span::styled(" (HEAD)", Style::default().add_modifier(Modifier::DIM)));
  //   }
  //
  //   if self.branch.upstream.is_some() {
  //     let upstream = self.branch.upstream.clone();
  //     parts.push(Span::styled(format!(" [{}]", upstream.unwrap().name), Style::default().add_modifier(Modifier::DIM)));
  //   }
  //
  //   text = text.spans(parts);
  //   ListItem::from(text)
  // }
}

impl ListItemWrapper<GitBranch> for BranchItem {
  fn new(branch: GitBranch) -> Self {
    BranchItem { branch, staged_for_deletion: false }
  }

  fn stage_for_deletion(&mut self, stage: bool) {
    self.staged_for_deletion = stage;
  }

  fn is_staged_for_deletion(&self) -> bool {
    self.staged_for_deletion
  }

  fn inner_item(&self) -> &GitBranch {
    &self.branch
  }

  fn render(&self) -> ListItem {
    let mut text = Line::default();
    let mut parts = Vec::new();
    let mut name = Span::styled(self.branch.name.clone(), Style::default());

    if self.staged_for_deletion {
      name = name.style(Style::default().fg(Color::Red));
    }
    // Removed creation styling logic
    parts.push(name);

    if self.branch.is_head {
      parts.push(Span::styled(" (HEAD)", Style::default().add_modifier(Modifier::DIM)));
    }

    if self.branch.upstream.is_some() {
      let upstream = self.branch.upstream.clone();
      parts.push(Span::styled(format!(" [{}]", upstream.unwrap().name), Style::default().add_modifier(Modifier::DIM)));
    }

    text = text.spans(parts);
    ListItem::from(text)
  }
}
