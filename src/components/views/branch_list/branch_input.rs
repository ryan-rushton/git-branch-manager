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
