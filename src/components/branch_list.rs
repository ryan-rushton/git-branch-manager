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
  git::repo::{GitBranch, GitRepo},
  tui::Frame,
};

#[derive(Default)]
pub struct GitBranchList {
  branches: Vec<GitBranch>,
  state: ListState,
}

impl GitBranchList {
  pub fn new(mut repo: GitRepo) -> Self {
    let branches = repo.local_branches().unwrap();
    GitBranchList { branches, state: ListState::default().with_selected(Some(0)) }
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
}

impl Component for GitBranchList {
  fn handle_key_events(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
    if key.code == KeyCode::Down {
      self.select_next()
    }
    if key.code == KeyCode::Up {
      self.select_previous()
    }
    Ok(None)
  }

  fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
    Ok(None)
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
    let list = List::new(self.get_render_items())
      .block(Block::default().title("List").borders(Borders::ALL))
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().add_modifier(Modifier::ITALIC).add_modifier(Modifier::BOLD))
      .highlight_symbol("→")
      .repeat_highlight_symbol(true);
    f.render_stateful_widget(list, area, &mut self.state);
    Ok(())
  }
}
