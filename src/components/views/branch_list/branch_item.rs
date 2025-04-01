use ratatui::{
  style::{Color, Modifier, Style},
  text::{Line, Span},
  widgets::ListItem,
};

use crate::git::types::GitBranch;

#[derive(Debug, Clone)]
pub struct BranchItem {
  pub branch: GitBranch,
  pub staged_for_creation: bool,
  pub staged_for_deletion: bool,
  pub is_valid_name: bool,
}

impl BranchItem {
  pub fn new(branch: GitBranch, is_valid_name: bool) -> Self {
    BranchItem { branch, staged_for_creation: false, staged_for_deletion: false, is_valid_name }
  }

  pub fn stage_for_deletion(&mut self, stage: bool) {
    self.staged_for_deletion = stage;
  }

  pub fn render(&self) -> ListItem {
    let mut text = Line::default();
    let mut parts = Vec::new();
    let mut name = Span::styled(self.branch.name.clone(), Style::default());

    if self.staged_for_deletion {
      name = name.style(Style::default().fg(Color::Red));
    }
    if self.staged_for_creation {
      name = name.style(Style::default().fg(if self.is_valid_name { Color::LightGreen } else { Color::LightRed }));
    }
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
