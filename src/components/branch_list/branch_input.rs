use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
  layout::Rect,
  prelude::Color,
  style::Style,
  widgets::{Block, Borders},
};
use tracing::{error, info};
use tui_textarea::{CursorMove, Input, TextArea};

use crate::{
  action::Action,
  git::git_repo::{GitBranch, GitRepo},
  tui::Frame,
};

#[derive(Debug, Default, Clone)]
pub struct InputState {
  pub value: Option<String>,
  pub is_valid: Option<bool>,
}

pub struct BranchInput {
  pub text_input: TextArea<'static>,
  pub input_state: InputState,
}

impl BranchInput {
  pub fn new() -> Self {
    BranchInput { text_input: TextArea::default(), input_state: InputState::default() }
  }

  pub fn init_style(&mut self) {
    self.text_input.set_style(Style::default().fg(Color::White));
    self.text_input.set_block(Block::default().borders(Borders::ALL));
  }

  fn get_text(&self) -> Option<String> {
    let input = String::from(self.text_input.lines().first()?.trim());
    if input.is_empty() {
      info!("BranchInput: get_text returned None - input was empty");
      return None;
    }
    info!("BranchInput: get_text returned text: {}", input);
    Some(input)
  }

  async fn validate_branch_name(&mut self, repo: &dyn GitRepo, current_branches: Vec<&GitBranch>) {
    if self.text_input.lines().is_empty() {
      info!("BranchInput: validate_branch_name skipped - no lines in text input");
      self.input_state.is_valid = Some(false);
      return;
    }

    let proposed_name = self.text_input.lines().first().unwrap().trim();
    if proposed_name.is_empty() {
      info!("BranchInput: validate_branch_name skipped - empty branch name");
      self.input_state.is_valid = Some(false);
      return;
    }

    info!("BranchInput: Validating branch name: '{}'", proposed_name);
    let is_valid = repo.validate_branch_name(proposed_name).await;
    let is_unique_name = !current_branches.iter().any(|b| b.name.eq(proposed_name));
    info!("BranchInput: Branch name validation result - valid: {:?}, unique: {}", is_valid, is_unique_name);

    match is_valid {
      Ok(valid) => {
        if !valid {
          info!("BranchInput: Branch name is not valid, setting red style");
          self.text_input.set_style(Style::default().fg(Color::LightRed));
          self.input_state.is_valid = Some(false);
        } else if !is_unique_name {
          info!("BranchInput: Branch name is not unique, setting red style");
          self.text_input.set_style(Style::default().fg(Color::LightRed));
          self.input_state.is_valid = Some(false);
        } else {
          info!("BranchInput: Branch name valid and unique, setting green style");
          self.text_input.set_style(Style::default().fg(Color::LightGreen));
          self.input_state.is_valid = Some(true);
        }
      },
      Err(e) => {
        error!("BranchInput: Branch name validation error: {}", e);
        self.text_input.set_style(Style::default().fg(Color::LightRed));
        self.input_state.is_valid = Some(false);
      },
    }
    info!("BranchInput: Final validation state: {:?}", self.input_state.is_valid);
  }

  pub async fn handle_key_event(
    &mut self,
    key_event: KeyEvent,
    repo: &dyn GitRepo,
    current_branches: Vec<&GitBranch>,
  ) -> Option<Action> {
    info!("BranchInput: Starting key event handling for: {:?}", key_event);
    match key_event {
      KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        info!("BranchInput: Handling Escape key");
        self.input_state.value = None;
        self.input_state.is_valid = None;
        info!("BranchInput: Reset input state to None");
        // purposely don't send the key, we want to delete the line
        self.text_input.move_cursor(CursorMove::Head);
        self.text_input.delete_line_by_end();
        info!("BranchInput: Cleared text input");
        info!("BranchInput: Returning EndInputMod action");
        Some(Action::EndInputMod)
      },
      KeyEvent { code: KeyCode::Enter, modifiers: _, kind: _, state: _ } => {
        info!("BranchInput: Handling Enter key");
        info!("BranchInput: Current validation state: {:?}", self.input_state.is_valid);
        info!("BranchInput: Current text value: {:?}", self.text_input.lines().first());

        if !self.input_state.is_valid.unwrap_or(false) {
          info!("BranchInput: Branch name not valid, ignoring Enter key");
          return None;
        }

        let new_branch_name = self.get_text();
        info!("BranchInput: Got branch name from text: {:?}", new_branch_name);

        // purposely don't send the key, we want to delete the line
        self.text_input.move_cursor(CursorMove::Head);
        self.text_input.delete_line_by_end();
        info!("BranchInput: Cleared text input");

        if let Some(name) = new_branch_name {
          info!("BranchInput: Returning CreateBranch action for branch '{}'", name);
          return Some(Action::CreateBranch(name));
        }

        info!("BranchInput: No branch name, returning EndInputMod action");
        Some(Action::EndInputMod)
      },
      _ => {
        info!("BranchInput: Handling other key event: {:?}", key_event);
        let changed = self.text_input.input(Input::from(key_event));
        info!("BranchInput: Text input changed: {}", changed);

        if changed {
          info!("BranchInput: Current text: {:?}", self.text_input.lines().first());
          info!("BranchInput: Validating branch name");
          self.validate_branch_name(repo, current_branches).await;
          let new_branch_name = self.get_text();
          if new_branch_name.is_some() {
            info!("BranchInput: Updating input state with new branch name: {:?}", new_branch_name);
            self.input_state.value = new_branch_name;
          }
          info!("BranchInput: Final validation state after key: {:?}", self.input_state.is_valid);
        }
        None
      },
    }
  }

  pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    f.render_widget(&self.text_input, area);
  }
}
