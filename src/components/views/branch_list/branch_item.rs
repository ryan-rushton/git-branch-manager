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

    if let Some(upstream) = self.branch.upstream.clone() {
      parts.push(Span::styled(
        format!(" [{}{}]", upstream.name, if upstream.gone { ": gone" } else { "" }),
        Style::default().add_modifier(Modifier::DIM),
      ));
    }

    text = text.spans(parts);
    ListItem::from(text)
  }
}

#[cfg(test)]
mod tests {
  use ratatui::widgets::ListItem;

  use super::*;
  use crate::git::types::GitBranch;

  #[test]
  fn test_new_branch_item() {
    let branch = GitBranch { name: "test-branch".to_string(), is_head: false, upstream: None };
    let branch_item = BranchItem::new(branch.clone(), true);

    assert_eq!(branch_item.branch.name, branch.name);
    assert!(branch_item.is_valid_name);
    assert!(!branch_item.staged_for_creation);
    assert!(!branch_item.staged_for_deletion);
  }

  #[test]
  fn test_stage_for_deletion() {
    let branch = GitBranch { name: "test-branch".to_string(), is_head: false, upstream: None };
    let mut branch_item = BranchItem::new(branch, true);

    branch_item.stage_for_deletion(true);
    assert!(branch_item.staged_for_deletion);

    branch_item.stage_for_deletion(false);
    assert!(!branch_item.staged_for_deletion);
  }

  #[test]
  fn test_render() {
    let branch = GitBranch { name: "test-branch".to_string(), is_head: false, upstream: None };
    let branch_item =
      BranchItem { branch, staged_for_creation: false, staged_for_deletion: false, is_valid_name: true };

    let rendered = branch_item.render();

    assert_eq!(rendered, ListItem::new("test-branch"));
  }

  #[test]
  fn test_render_staged_for_creation_with_valid_name() {
    let branch = GitBranch { name: "test-branch".to_string(), is_head: false, upstream: None };
    let branch_item = BranchItem { branch, staged_for_creation: true, staged_for_deletion: false, is_valid_name: true };

    let rendered = branch_item.render();

    assert_eq!(rendered, ListItem::new(Span::from("test-branch").style(Style::default().fg(Color::LightGreen))));
  }

  #[test]
  fn test_render_staged_for_creation_with_invalid_name() {
    let branch = GitBranch { name: "test-branch".to_string(), is_head: false, upstream: None };
    let branch_item =
      BranchItem { branch, staged_for_creation: true, staged_for_deletion: false, is_valid_name: false };

    let rendered = branch_item.render();

    assert_eq!(rendered, ListItem::new(Span::from("test-branch").style(Style::default().fg(Color::LightRed))));
  }

  #[test]
  fn test_render_head_with_remote_gone() {
    let branch = GitBranch {
      name: "test-branch".to_string(),
      is_head: true,
      upstream: Some(crate::git::types::GitRemoteBranch { name: "origin/test-branch".to_string(), gone: true }),
    };
    let branch_item =
      BranchItem { branch, staged_for_creation: false, staged_for_deletion: false, is_valid_name: true };

    let rendered = branch_item.render();

    assert_eq!(
      rendered,
      ListItem::new(Line::from_iter([
        Span::from("test-branch"),
        Span::from(" (HEAD)").style(Style::default().add_modifier(Modifier::DIM)),
        Span::from(" [origin/test-branch: gone]").style(Style::default().add_modifier(Modifier::DIM))
      ]))
    );
  }

  #[test]
  fn test_render_head_with_remote_staged_for_deletion() {
    let branch = GitBranch {
      name: "test-branch".to_string(),
      is_head: true,
      upstream: Some(crate::git::types::GitRemoteBranch { name: "origin/test-branch".to_string(), gone: false }),
    };
    let branch_item = BranchItem { branch, staged_for_creation: false, staged_for_deletion: true, is_valid_name: true };

    let rendered = branch_item.render();

    assert_eq!(
      rendered,
      ListItem::new(Line::from_iter([
        Span::from("test-branch").style(Style::default().fg(Color::Red)),
        Span::from(" (HEAD)").style(Style::default().add_modifier(Modifier::DIM)),
        Span::from(" [origin/test-branch]").style(Style::default().add_modifier(Modifier::DIM))
      ]))
    );
  }
}
