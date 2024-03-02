use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span, Text},
  widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use tui_textarea::{CursorMove, Input, TextArea};

use crate::{
  action::Action,
  components::Component,
  git::repo::{GitBranch, GitRepo},
  tui::Frame,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct BranchItem {
  branch: GitBranch,
  staged_for_deletion: bool,
}

impl BranchItem {
  pub fn render(&self) -> ListItem {
    let mut text = self.branch.name.clone();
    if self.branch.is_head {
      text += " (HEAD)";
    }
    let mut item = ListItem::new(text);
    if self.staged_for_deletion {
      item = item.style(Color::Red);
    }
    item
  }

  pub fn stage_for_deletion(&mut self, stage: bool) {
    self.staged_for_deletion = stage;
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
  Selection,
  Input,
}

struct InputState {
  value: Option<String>,
  is_valid: Option<bool>,
}

pub struct GitBranchList {
  mode: Mode,
  repo: GitRepo,
  error: Option<String>,
  // List state
  branches: Vec<BranchItem>,
  list_state: ListState,
  // Input state
  text_input: TextArea<'static>,
  input_state: InputState,
}

impl Default for GitBranchList {
  fn default() -> Self {
    GitBranchList::new()
  }
}

impl GitBranchList {
  pub fn new() -> Self {
    let repo = GitRepo::from_cwd().unwrap();
    let branches: Vec<BranchItem> = repo
      .local_branches()
      .unwrap()
      .iter()
      .map(|branch| BranchItem { branch: branch.clone(), staged_for_deletion: false })
      .collect();
    let text_input = TextArea::default();
    GitBranchList {
      repo,
      mode: Mode::Selection,
      error: None,
      branches,
      list_state: ListState::default().with_selected(Some(0)),
      text_input,
      input_state: InputState { value: None, is_valid: None },
    }
  }

  pub fn select_previous(&mut self) {
    if self.list_state.selected().is_none() {
      self.list_state.select(Some(0));
    }

    let selected = self.list_state.selected().unwrap();
    let final_index = self.branches.len() - 1;

    if selected == 0 {
      self.list_state.select(Some(final_index));
      return;
    }
    self.list_state.select(Some(selected - 1))
  }

  pub fn select_next(&mut self) {
    if self.list_state.selected().is_none() {
      self.list_state.select(Some(0));
    }

    let selected = self.list_state.selected().unwrap();
    let final_index = self.branches.len() - 1;

    if selected == final_index {
      self.list_state.select(Some(0));
      return;
    }
    self.list_state.select(Some(selected + 1))
  }

  fn get_selected_branch(&self) -> Option<&BranchItem> {
    let selected_index = self.list_state.selected()?;
    self.branches.get(selected_index)
  }

  fn checkout_selected(&mut self) -> Result<(), git2::Error> {
    let maybe_selected = self.get_selected_branch();
    if maybe_selected.is_none() {
      return Ok(());
    }
    let name_to_checkout = maybe_selected.unwrap().branch.name.clone();
    self.repo.checkout_branch_from_name(&name_to_checkout)?;
    for existing_branch in self.branches.iter_mut() {
      existing_branch.branch.is_head = existing_branch.branch.name == name_to_checkout;
    }
    Ok(())
  }

  pub fn stage_selected_for_deletion(&mut self, stage: bool) {
    if self.list_state.selected().is_none() {
      return;
    }
    let selected_index = self.list_state.selected().unwrap();
    let maybe_selected = self.branches.get_mut(selected_index);
    if maybe_selected.is_none() {
      return;
    }
    let selected = maybe_selected.unwrap();
    if selected.branch.is_head {
      return;
    }
    selected.stage_for_deletion(stage);
  }

  pub fn deleted_selected(&mut self) -> Result<(), git2::Error> {
    if self.list_state.selected().is_none() {
      return Ok(());
    }
    let selected_index = self.list_state.selected().unwrap();
    let selected = self.branches.get(selected_index);
    if selected.is_none() {
      return Ok(());
    }
    let delete_result = self.repo.delete_branch(&selected.unwrap().branch);
    if delete_result.is_err() {
      return Ok(());
    }
    self.branches.remove(selected_index);
    if selected_index >= self.branches.len() {
      self.list_state.select(Some(selected_index - 1));
    }
    Ok(())
  }

  pub fn delete_staged_branches(&mut self) -> Result<(), git2::Error> {
    let mut indexes_to_delete: Vec<usize> = Vec::new();

    for branch_index in 0..self.branches.len() {
      let branch_item = &self.branches[branch_index];
      if !branch_item.staged_for_deletion {
        continue;
      }
      let del_result = self.repo.delete_branch(&branch_item.branch);
      if del_result.is_ok() {
        indexes_to_delete.push(branch_index);
      } else {
        // TODO communicate deletion error
      }
    }

    // Sort and reverse, so we remove branches starting from the end,
    // which means we don't need to worry about changing array positions.
    indexes_to_delete.reverse();
    for index in indexes_to_delete {
      self.branches.remove(index);
    }
    Ok(())
  }

  fn validate_branch_name(&mut self) {
    if self.text_input.lines().first().is_none() {
      return;
    }
    let proposed_name = self.text_input.lines().first().unwrap();
    let is_valid = self.repo.validate_branch_name(proposed_name);
    if is_valid.is_err() || !is_valid.unwrap() {
      self.text_input.set_style(Style::default().fg(Color::LightRed));
      self.input_state.is_valid = Some(false);
      return;
    }
    self.text_input.set_style(Style::default().fg(Color::LightGreen));
    self.input_state.is_valid = Some(true);
  }

  fn create_branch(&mut self, name: String) -> Result<(), git2::Error> {
    let branch = GitBranch { name: name.clone(), is_head: false };
    self.repo.create_branch(&branch)?;
    self.branches.push(BranchItem { branch, staged_for_deletion: false });
    self.branches.sort_by(|a, b| a.branch.name.cmp(&b.branch.name));
    self.repo.checkout_branch_from_name(&name)?;
    for existing_branch in self.branches.iter_mut() {
      existing_branch.branch.is_head = existing_branch.branch.name == name;
    }
    let created_index = self.branches.iter().position(|b| b.branch.name == name);
    self.list_state.select(created_index);
    Ok(())
  }

  fn get_first_input_line(&self) -> Option<String> {
    Some(String::from(self.text_input.lines().first()?))
  }

  fn handle_input_key_event(&mut self, key_event: KeyEvent) -> Option<Action> {
    match key_event {
      KeyEvent { code: KeyCode::Esc, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        self.mode = Mode::Selection;
        self.input_state.value = None;
        // purposely don't send the key, we want to delete the line
        self.text_input.move_cursor(CursorMove::Head);
        self.text_input.delete_line_by_end();
        None
      },
      KeyEvent { code: KeyCode::Enter, modifiers: _, kind: _, state: _ } => {
        if self.input_state.is_valid.is_some() && !self.input_state.is_valid.unwrap() {
          // TODO report error
          return None;
        }
        self.mode = Mode::Selection;
        let new_branch_name = self.get_first_input_line();
        // purposely don't send the key, we want to delete the line
        self.text_input.move_cursor(CursorMove::Head);
        self.text_input.delete_line_by_end();
        if let Some(name) = new_branch_name {
          return Some(Action::CreateBranch(name));
        }

        Some(Action::EndInputMod)
      },
      _ => {
        if self.text_input.input(Input::from(key_event)) {
          self.validate_branch_name();
          let new_branch_name = self.get_first_input_line();
          if new_branch_name.is_some() {
            self.input_state.value = new_branch_name;
          }
        }
        Some(Action::EndInputMod)
      },
    }
  }

  fn maybe_handle_git_error(&mut self, err: Option<git2::Error>) {
    if err.is_some() {
      self.error = Some(format!("{}", err.unwrap().message()));
    }
  }

  fn render_list(&mut self, f: &mut Frame<'_>, area: Rect) {
    let render_items: Vec<ListItem> = self.branches.iter().map(|git_branch| git_branch.render()).collect();
    let list = List::new(render_items)
      .block(Block::default().title("Local Branches").borders(Borders::ALL))
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().add_modifier(Modifier::BOLD))
      .highlight_symbol("→")
      .repeat_highlight_symbol(true);

    f.render_stateful_widget(list, area, &mut self.list_state);
  }

  fn render_input(&mut self, f: &mut Frame<'_>, area: Rect) {
    let input = self.text_input.widget();
    f.render_widget(input, area);
  }

  fn render_error(&mut self, f: &mut Frame<'_>, area: Rect) {
    if self.error.is_none() {
      return;
    }
    let error_message = self.error.as_ref().unwrap().clone();
    let text = Text::from(error_message);
    let component = Paragraph::new(text).block(Block::default()).style(Style::from(Color::Red));
    f.render_widget(component, area);
  }

  fn render_footer(&mut self, f: &mut Frame<'_>, area: Rect) {
    let mut commands = vec![Span::raw("q: Quit")];
    commands.push(Span::raw(" | ⇧ + c: Create branch"));
    let selected = self.get_selected_branch();
    if selected.is_some() && selected.unwrap().staged_for_deletion {
      commands.push(Span::raw(" | d: Delete"));
      commands.push(Span::raw(" | ⇧ + d: Unstage for deletion"));
    } else if selected.is_some() && !selected.unwrap().branch.is_head {
      commands.push(Span::raw(" | d: Stage for deletion"));
    } else if selected.is_some() {
      commands.push(Span::raw(" | c: Checkout"));
    }
    commands.push(Span::raw(" | ^ + d: Delete all staged branches"));
    let footer = Line::from(commands);
    f.render_widget(footer, area);
  }
}

impl Component for GitBranchList {
  fn handle_key_events(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
    if self.mode == Mode::Input {
      return Ok(Some(Action::UpdateNewBranchName(key)));
    }
    match key {
      KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Ok(Some(Action::SelectNextBranch))
      },
      KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Ok(Some(Action::SelectPreviousBranch))
      },
      KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::SHIFT, kind: _, state: _ } => {
        Ok(Some(Action::InitNewBranch))
      },
      KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Ok(Some(Action::CheckoutSelectedBranch))
      },
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::SHIFT, kind: _, state: _ } => {
        Ok(Some(Action::UnstageBranchForDeletion))
      },
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, kind: _, state: _ } => {
        Ok(Some(Action::DeleteStagedBranches))
      },
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        if self.get_selected_branch().is_none() {
          return Ok(None);
        }
        let selected = self.get_selected_branch().unwrap();
        if selected.staged_for_deletion {
          return Ok(Some(Action::DeleteBranch));
        }
        Ok(Some(Action::StageBranchForDeletion))
      },
      _ => Ok(None),
    }
  }

  fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
    match action {
      Action::SelectPreviousBranch => {
        self.select_previous();
        Ok(None)
      },
      Action::SelectNextBranch => {
        self.select_next();
        Ok(None)
      },
      Action::InitNewBranch => {
        self.mode = Mode::Input;
        self.text_input.set_style(Style::default().fg(Color::White));
        self.text_input.set_block(Block::default().borders(Borders::ALL));
        Ok(Some(Action::StartInputMode))
      },
      Action::UpdateNewBranchName(key_event) => Ok(self.handle_input_key_event(key_event)),
      Action::CheckoutSelectedBranch => {
        let result = self.checkout_selected();
        self.maybe_handle_git_error(result.err());
        Ok(None)
      },
      Action::CreateBranch(name) => {
        self.mode = Mode::Selection;
        let result = self.create_branch(name);
        self.maybe_handle_git_error(result.err());
        Ok(Some(Action::EndInputMod))
      },
      Action::StageBranchForDeletion => {
        self.stage_selected_for_deletion(true);
        Ok(None)
      },
      Action::UnstageBranchForDeletion => {
        self.stage_selected_for_deletion(false);
        Ok(None)
      },
      Action::DeleteBranch => {
        let result = self.deleted_selected();
        self.maybe_handle_git_error(result.err());
        Ok(None)
      },
      Action::DeleteStagedBranches => {
        let result = self.delete_staged_branches();
        self.maybe_handle_git_error(result.err());
        Ok(None)
      },
      _ => Ok(None),
    }
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
    if self.mode == Mode::Input {
      let layout =
        Layout::new(Direction::Vertical, [Constraint::Fill(1), Constraint::Length(3), Constraint::Length(1)])
          .margin(1)
          .split(area);
      self.render_list(f, layout[0]);
      self.render_input(f, layout[1]);
      self.render_footer(f, layout[2]);
      return Ok(());
    }

    if self.error.is_some() {
      let layout =
        Layout::new(Direction::Vertical, [Constraint::Fill(1), Constraint::Length(2), Constraint::Length(1)])
          .margin(1)
          .split(area);
      self.render_list(f, layout[0]);
      self.render_error(f, layout[1]);
      self.render_footer(f, layout[2]);
      return Ok(());
    }

    let layout = Layout::new(Direction::Vertical, [Constraint::Fill(1), Constraint::Length(1)]).margin(1).split(area);
    self.render_list(f, layout[0]);
    self.render_footer(f, layout[1]);

    Ok(())
  }
}
