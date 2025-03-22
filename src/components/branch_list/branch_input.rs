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
      return None;
    }
    Some(input)
  }

  async fn validate_branch_name(&mut self, repo: &dyn GitRepo, current_branches: Vec<&GitBranch>) {
    if self.text_input.lines().is_empty() {
      self.input_state.is_valid = Some(false);
      return;
    }

    let proposed_name = self.text_input.lines().first().unwrap().trim();
    if proposed_name.is_empty() {
      self.input_state.is_valid = Some(false);
      return;
    }

    let is_valid = repo.validate_branch_name(proposed_name).await;
    let is_unique_name = !current_branches.iter().any(|b| b.name.eq(proposed_name));

    match is_valid {
      Ok(valid) => {
        if !valid {
          self.text_input.set_style(Style::default().fg(Color::LightRed));
          self.input_state.is_valid = Some(false);
        } else if !is_unique_name {
          self.text_input.set_style(Style::default().fg(Color::LightRed));
          self.input_state.is_valid = Some(false);
        } else {
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
  }

  pub async fn handle_key_event(
    &mut self,
    key_event: KeyEvent,
    repo: &dyn GitRepo,
    current_branches: Vec<&GitBranch>,
  ) -> Option<Action> {
    match key_event {
      KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        self.input_state.value = None;
        self.input_state.is_valid = None;
        self.text_input.move_cursor(CursorMove::Head);
        self.text_input.delete_line_by_end();
        Some(Action::EndInputMod)
      },
      KeyEvent { code: KeyCode::Enter, modifiers: _, kind: _, state: _ } => {
        if !self.input_state.is_valid.unwrap_or(false) {
          return None;
        }

        let new_branch_name = self.get_text();
        self.text_input.move_cursor(CursorMove::Head);
        self.text_input.delete_line_by_end();

        if let Some(name) = new_branch_name {
          info!("BranchInput: Creating branch '{}'", name);
          return Some(Action::CreateBranch(name));
        }
        Some(Action::EndInputMod)
      },
      _ => {
        let changed = self.text_input.input(Input::from(key_event));
        if changed {
          self.validate_branch_name(repo, current_branches).await;
          let new_branch_name = self.get_text();
          if new_branch_name.is_some() {
            self.input_state.value = new_branch_name;
          }
        }
        None
      },
    }
  }

  pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    f.render_widget(&self.text_input, area);
  }
}
