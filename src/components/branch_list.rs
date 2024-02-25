use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span},
  widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::{
  action::Action,
  components::Component,
  error::Error,
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
    let mut item = ListItem::new(self.branch.name.clone());
    if self.staged_for_deletion {
      item = item.style(Color::Red);
    }
    item
  }

  pub fn stage_for_deletion(&mut self, stage: bool) {
    self.staged_for_deletion = stage;
  }
}

pub struct GitBranchList {
  repo: GitRepo,
  branches: Vec<BranchItem>,
  state: ListState,
}

impl Default for GitBranchList {
  fn default() -> Self {
    Self::new()
  }
}

impl GitBranchList {
  pub fn new() -> Self {
    let repo = GitRepo::from_cwd().unwrap();
    let branches = repo
      .local_branches()
      .unwrap()
      .iter()
      .map(|branch| BranchItem { branch: branch.clone(), staged_for_deletion: false })
      .collect();
    GitBranchList { repo, branches, state: ListState::default().with_selected(Some(0)) }
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

  fn get_selected_branch(&self) -> Option<&BranchItem> {
    let selected_index = self.state.selected()?;
    self.branches.get(selected_index)
  }

  pub fn stage_selected_for_deletion(&mut self, stage: bool) -> Result<(), Error> {
    if self.state.selected().is_none() {
      return Ok(());
    }
    let selected_index = self.state.selected().unwrap();
    let selected = self.branches.get_mut(selected_index);
    if selected.is_none() {
      return Ok(());
    }
    selected.unwrap().stage_for_deletion(stage);
    Ok(())
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
    let delete_result = self.repo.delete_branch(&selected.unwrap().branch);
    if delete_result.is_err() {
      return Ok(());
    }
    self.branches.remove(selected_index);
    Ok(())
  }

  pub fn delete_marked_branches(&mut self) -> Result<(), Error> {
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
}

impl Component for GitBranchList {
  fn handle_key_events(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
    match key {
      KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Ok(Some(Action::SelectNextBranch))
      },
      KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Ok(Some(Action::SelectPreviousBranch))
      },
      KeyEvent { code: KeyCode::Char('d') | KeyCode::Char('D'), modifiers: KeyModifiers::SHIFT, kind: _, state: _ } => {
        Ok(Some(Action::UnstageBranchForDeletion))
      },
      KeyEvent { code: KeyCode::Char('d'), modifiers: KeyModifiers::CONTROL, kind: _, state: _ } => {
        Ok(Some(Action::DeleteStagedBranches))
      },
      KeyEvent { code: KeyCode::Char('d'), modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
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
      Action::StageBranchForDeletion => {
        self.stage_selected_for_deletion(true)?;
        Ok(None)
      },
      Action::UnstageBranchForDeletion => {
        self.stage_selected_for_deletion(false)?;
        Ok(None)
      },
      Action::DeleteBranch => {
        self.deleted_selected()?;
        Ok(None)
      },
      Action::DeleteStagedBranches => {
        self.delete_marked_branches()?;
        Ok(None)
      },
      _ => Ok(None),
    }
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
    let layout = Layout::new(Direction::Vertical, [Constraint::Fill(1), Constraint::Length(1)]).margin(1).split(area);

    let render_items: Vec<ListItem> = self.branches.iter().map(|git_branch| git_branch.render()).collect();
    let list = List::new(render_items)
      .block(Block::default().title("Local Branches").borders(Borders::ALL))
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().add_modifier(Modifier::ITALIC).add_modifier(Modifier::BOLD))
      .highlight_symbol("→")
      .repeat_highlight_symbol(true);

    let mut commands = vec![Span::raw("q: Quit")];
    if self.get_selected_branch().is_some() && self.get_selected_branch().unwrap().staged_for_deletion {
      commands.push(Span::raw(" | d: Delete"));
      commands.push(Span::raw(" | ⇧ + d: Unmark for deletion"));
    } else {
      commands.push(Span::raw(" | d: Stage for deletion"));
    }
    commands.push(Span::raw(" | ⇧ + ←: Delete all staged branches"));
    let footer = Line::from(commands);

    f.render_stateful_widget(list, layout[0], &mut self.state);
    f.render_widget(footer, layout[1]);

    Ok(())
  }
}
