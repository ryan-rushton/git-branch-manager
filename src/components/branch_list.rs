use color_eyre::owo_colors::OwoColorize;
use crossterm::event::{KeyCode, KeyEvent};
use git2::Repository;
use ratatui::{
  layout::{Alignment, Rect},
  style::{Color, Modifier, Style},
  widgets::{block, Block, Borders, List, ListState},
};

use crate::{
  action::Action,
  components::{home::Home, Component},
  error::Error,
  git::repo::{GitBranch, GitRepo},
  tui::Frame,
};

pub struct GitBranchList {
  repo: GitRepo,
  branches: Vec<GitBranch>,
  state: ListState,
}

impl Default for GitBranchList {
  fn default() -> Self {
    Self::new()
  }
}

impl GitBranchList {
  pub fn new() -> Self {
    let mut repo = GitRepo::from_cwd().unwrap();
    let branches = repo.local_branches().unwrap();
    GitBranchList { repo, branches, state: ListState::default().with_selected(Some(0)) }
  }

  pub fn get_render_items(&mut self) -> Vec<String> {
    return self.branches.iter().map(|git_branch| git_branch.name.clone()).collect();
  }

  pub fn select_previous(&mut self) {
    if self.state.selected().is_none() {
      self.state.select(Some(0));
    }

    let selected = self.state.selected().unwrap();
    let final_index = self.branches.len() - 1;

    if selected == 0 {
      self.state.select(Some(final_index));
      return;
    }
    self.state.select(Some(selected - 1))
  }

  pub fn select_next(&mut self) {
    if self.state.selected().is_none() {
      self.state.select(Some(0));
    }

    let selected = self.state.selected().unwrap();
    let final_index = self.branches.len() - 1;

    if selected == final_index {
      self.state.select(Some(0));
      return;
    }
    self.state.select(Some(selected + 1))
  }

  pub fn deleted_selected(&mut self) -> Result<(), Error> {
    if self.state.selected().is_none() {
      return Ok(());
    }
    let selected_index = self.state.selected().unwrap();
    let selected = self.branches.get(selected_index);
    if selected.is_none() {
      return Ok(());
    }
    let delete_result = self.repo.delete_branch(selected.unwrap());
    if delete_result.is_err() {
      return Ok(());
    }
    self.branches.remove(selected_index);
    Ok(())
  }
}

impl Component for GitBranchList {
  fn handle_key_events(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
    if key.code == KeyCode::Down {
      self.select_next()
    }
    if key.code == KeyCode::Up {
      self.select_previous()
    }
    if key.code == KeyCode::Delete || key.code == KeyCode::Backspace {
      self.deleted_selected()?
    }
    Ok(None)
  }

  fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
    Ok(None)
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
    let list = List::new(self.get_render_items())
      .block(Block::default().title("Local Branches").borders(Borders::ALL))
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().add_modifier(Modifier::ITALIC).add_modifier(Modifier::BOLD))
      .highlight_symbol("→")
      .repeat_highlight_symbol(true);
    f.render_stateful_widget(list, area, &mut self.state);
    Ok(())
  }
}
