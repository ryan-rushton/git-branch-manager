use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use tokio::task::block_in_place;

use crate::{
  action::Action,
  components::common::text_input::{InputState, TextInput},
  git::types::{GitBranch, GitRepo},
  tui::Frame,
};

#[derive(Debug, Default)]
pub struct BranchInput {
  text_input: TextInput,
}

impl BranchInput {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn init_style(&mut self) {
    self.text_input.init_style();
  }

  pub fn get_state(&self) -> InputState {
    self.text_input.input_state.clone()
  }

  fn validate_branch_name(proposed_name: &str, repo: &dyn GitRepo, current_branches: &[&GitBranch]) -> bool {
    let is_valid =
      block_in_place(|| tokio::runtime::Handle::current().block_on(repo.validate_branch_name(proposed_name)))
        .unwrap_or(false);
    let is_unique_name = !current_branches.iter().any(|b| b.name.eq(proposed_name));
    is_valid && is_unique_name
  }

  pub async fn handle_key_event(
    &mut self,
    key_event: KeyEvent,
    repo: &dyn GitRepo,
    current_branches: Vec<&GitBranch>,
  ) -> Option<Action> {
    let validate_fn = |proposed_name: &str| BranchInput::validate_branch_name(proposed_name, repo, &current_branches);

    self.text_input.handle_key_event(key_event, validate_fn).map(|action| {
      match action {
        Action::InputSubmitted(text) => Action::CreateBranch(text),
        _ => action,
      }
    })
  }

  pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    self.text_input.render(f, area);
  }
}

#[cfg(test)]
mod tests {
  use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

  use super::*;
  use crate::git::{mock_git_repo::MockGitRepo, types::GitBranch};

  #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
  async fn test_handle_key_event_submit() {
    let mut branch_input = BranchInput::new();
    branch_input.text_input.text_input.insert_str("new-branch");

    let repo = MockGitRepo;
    let main_branch = GitBranch::new("main".to_string());
    let current_branches = vec![&main_branch];

    let action = branch_input
      .handle_key_event(
        KeyEvent {
          code: KeyCode::Enter,
          modifiers: KeyModifiers::NONE,
          kind: crossterm::event::KeyEventKind::Press,
          state: crossterm::event::KeyEventState::NONE,
        },
        &repo,
        current_branches,
      )
      .await;

    assert_eq!(action, Some(Action::CreateBranch("new-branch".to_string())));
  }

  #[tokio::test]
  async fn test_handle_key_event_escape() {
    let mut branch_input = BranchInput::new();
    branch_input.text_input.text_input.insert_str("some input");

    let repo = MockGitRepo;
    let current_branches = vec![];

    let action = branch_input
      .handle_key_event(
        KeyEvent {
          code: KeyCode::Esc,
          modifiers: KeyModifiers::NONE,
          kind: crossterm::event::KeyEventKind::Press,
          state: crossterm::event::KeyEventState::NONE,
        },
        &repo,
        current_branches,
      )
      .await;

    assert_eq!(action, Some(Action::EndInputMode));
    assert!(branch_input.text_input.get_text().is_none());
  }

  #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
  async fn test_handle_key_event_char() {
    let mut branch_input = BranchInput::new();
    branch_input.text_input.text_input.insert_str("tes");

    let repo = MockGitRepo;
    let current_branches = vec![];

    branch_input
      .handle_key_event(
        KeyEvent {
          code: KeyCode::Char('t'),
          modifiers: KeyModifiers::NONE,
          kind: crossterm::event::KeyEventKind::Press,
          state: crossterm::event::KeyEventState::NONE,
        },
        &repo,
        current_branches,
      )
      .await;

    assert_eq!(branch_input.text_input.get_text(), Some("test".to_string()));
  }
}
