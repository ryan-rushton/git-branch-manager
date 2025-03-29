use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::Text,
  widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use tokio::{sync::mpsc::UnboundedSender, task::spawn};
use tracing::{error, info};

use crate::{
  action::Action,
  components::{
    Component,
    branch_list::{branch_input::BranchInput, branch_item::BranchItem, instruction_footer::InstructionFooter},
  },
  error::Error,
  git::git_repo::{GitBranch, GitRepo},
  tui::Frame,
};

mod branch_input;
mod branch_item;
mod instruction_footer;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
  Selection,
  Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadingOperation {
  None,
  LoadingBranches,
  CheckingOut,
  Creating,
  Deleting,
}

// Shared state that can be accessed from async blocks
#[derive(Clone)]
struct SharedState {
  error: Arc<Mutex<Option<String>>>,
  loading: Arc<Mutex<LoadingOperation>>,
  branches: Arc<Mutex<Vec<BranchItem>>>,
  selected_index: Arc<Mutex<usize>>,
  action_tx: Arc<Mutex<Option<UnboundedSender<Action>>>>,
}

impl SharedState {
  fn new() -> Self {
    SharedState {
      error: Arc::new(Mutex::new(None)),
      loading: Arc::new(Mutex::new(LoadingOperation::None)),
      branches: Arc::new(Mutex::new(Vec::new())),
      selected_index: Arc::new(Mutex::new(0)),
      action_tx: Arc::new(Mutex::new(None)),
    }
  }

  fn set_error(&self, err: Option<Error>) {
    if let Some(error) = err {
      let mut error_guard = self.error.lock().unwrap();
      *error_guard = Some(error.to_string());
    }
  }

  fn set_loading(&self, op: LoadingOperation) {
    let mut loading_guard = self.loading.lock().unwrap();
    *loading_guard = op;
  }

  fn send_render(&self) {
    if let Some(tx) = self.action_tx.lock().unwrap().as_ref() {
      let _ = tx.send(Action::Render);
    }
  }

  fn update_branches(&self, new_branches: Vec<BranchItem>) {
    let mut branches_guard = self.branches.lock().unwrap();
    *branches_guard = new_branches;
  }

  fn get_branches(&self) -> Vec<BranchItem> {
    self.branches.lock().unwrap().clone()
  }

  fn update_selected_index(&self, index: usize) {
    let mut index_guard = self.selected_index.lock().unwrap();
    *index_guard = index;
  }

  fn get_selected_index(&self) -> usize {
    *self.selected_index.lock().unwrap()
  }
}

pub struct BranchList {
  mode: Mode,
  repo: Arc<dyn GitRepo>,
  // Moved to shared state
  shared_state: SharedState,
  // Local cached copies for rendering
  error: Option<String>,
  loading: LoadingOperation,
  action_tx: Option<UnboundedSender<Action>>,
  branches: Vec<BranchItem>,
  list_state: ListState,
  selected_index: usize,
  // Components
  branch_input: BranchInput,
  instruction_footer: InstructionFooter,
}

impl BranchList {
  pub fn new(repo: Arc<dyn GitRepo>) -> Self {
    let shared_state = SharedState::new();

    BranchList {
      repo,
      mode: Mode::Selection,
      shared_state,
      error: None,
      loading: LoadingOperation::None,
      action_tx: None,
      branches: Vec::new(),
      list_state: ListState::default(),
      selected_index: 0,
      branch_input: BranchInput::new(),
      instruction_footer: InstructionFooter::default(),
    }
  }

  pub fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<(), Error> {
    self.action_tx = Some(tx.clone());
    *self.shared_state.action_tx.lock().unwrap() = Some(tx);
    Ok(())
  }

  // Sync UI state with shared state
  fn sync_state_for_render(&mut self) {
    self.error = self.shared_state.error.lock().unwrap().clone();
    self.loading = *self.shared_state.loading.lock().unwrap();
    self.branches = self.shared_state.get_branches();
    self.selected_index = self.shared_state.get_selected_index();
  }

  pub fn load_branches(&self) -> impl FnOnce() {
    let state = self.shared_state.clone();
    let repo_clone = self.repo.clone(); // Assuming repo can be cloned, might need a different approach

    move || {
      state.set_loading(LoadingOperation::LoadingBranches);
      state.send_render();

      let future = async move {
        let branches_result = repo_clone.local_branches().await;

        match branches_result {
          Ok(branches) => {
            let branch_items = branches.iter().map(|branch| BranchItem::new(branch.clone(), true)).collect();
            state.update_branches(branch_items);
            state.set_loading(LoadingOperation::None);
            state.send_render();
          },
          Err(err) => {
            error!("{}", err);
            state.set_error(Some(err));
            state.set_loading(LoadingOperation::None);
            state.send_render();
          },
        }
      };

      spawn(future);
    }
  }

  pub fn clear_error(&mut self) {
    *self.shared_state.error.lock().unwrap() = None;
    self.error = None;
  }

  pub fn select_previous(&mut self) {
    let branches = self.shared_state.branches.lock().unwrap();
    let mut selected_idx = self.shared_state.selected_index.lock().unwrap();

    if *selected_idx == 0 {
      *selected_idx = branches.len() - 1;
      return;
    }
    if *selected_idx >= branches.len() {
      *selected_idx = branches.len() - 1;
      return;
    }
    *selected_idx -= 1;

    // Update local copy for rendering
    self.selected_index = *selected_idx;
  }

  pub fn select_next(&mut self) {
    let branches = self.shared_state.branches.lock().unwrap();
    let mut selected_idx = self.shared_state.selected_index.lock().unwrap();

    if *selected_idx == branches.len() - 1 {
      *selected_idx = 0;
      return;
    }
    if *selected_idx >= branches.len() {
      *selected_idx = 0;
      return;
    }
    *selected_idx += 1;

    // Update local copy for rendering
    self.selected_index = *selected_idx;
  }

  fn get_selected_branch(&self) -> Option<&BranchItem> {
    self.branches.get(self.selected_index)
  }

  fn checkout_selected(&self) -> impl FnOnce() {
    let state = self.shared_state.clone();
    let repo_clone = self.repo.clone();

    move || {
      let branches = state.get_branches();
      let selected_idx = state.get_selected_index();

      let maybe_selected = branches.get(selected_idx);
      if maybe_selected.is_none() {
        return;
      }

      let name_to_checkout = maybe_selected.unwrap().branch.name.clone();
      state.set_loading(LoadingOperation::CheckingOut);
      state.send_render();

      let future = async move {
        let checkout_result = repo_clone.checkout_branch_from_name(&name_to_checkout).await;

        if let Err(err) = checkout_result {
          error!("{}", err);
          state.set_error(Some(err));
          state.set_loading(LoadingOperation::None);
          state.send_render();
          return;
        }

        let mut branches = state.get_branches();
        for existing_branch in branches.iter_mut() {
          existing_branch.branch.is_head = existing_branch.branch.name == name_to_checkout;
        }

        state.update_branches(branches);
        state.set_loading(LoadingOperation::None);
        state.send_render();
      };

      spawn(future);
    }
  }

  pub fn stage_selected_for_deletion(&mut self, stage: bool) {
    let selected_idx = self.shared_state.get_selected_index();
    let mut branches = self.shared_state.get_branches();

    let maybe_selected = branches.get_mut(selected_idx);
    if maybe_selected.is_none() {
      return;
    }

    let selected = maybe_selected.unwrap();
    if selected.branch.is_head {
      return;
    }

    selected.stage_for_deletion(stage);
    self.shared_state.update_branches(branches);

    // Update local copy for rendering
    self.branches = self.shared_state.get_branches();
  }

  pub fn deleted_selected(&self) -> impl FnOnce() {
    let state = self.shared_state.clone();
    let repo_clone = self.repo.clone();

    move || {
      let branches = state.get_branches();
      let selected_idx = state.get_selected_index();

      let selected = branches.get(selected_idx);
      if selected.is_none() {
        return;
      }

      state.set_loading(LoadingOperation::Deleting);
      state.send_render();

      let selected_branch = selected.unwrap().branch.clone();

      let future = async move {
        let delete_result = repo_clone.delete_branch(&selected_branch).await;

        if let Err(err) = delete_result {
          error!("{}", err);
          state.set_error(Some(err));
          state.set_loading(LoadingOperation::None);
          state.send_render();
          return;
        }

        let mut branches = state.get_branches();
        branches.remove(selected_idx);

        let mut new_selected_idx = selected_idx;
        if new_selected_idx >= branches.len() && !branches.is_empty() {
          new_selected_idx -= 1;
        }

        state.update_branches(branches);
        state.update_selected_index(new_selected_idx);
        state.set_loading(LoadingOperation::None);
        state.send_render();
      };

      spawn(future);
    }
  }

  pub fn delete_staged_branches(&self) -> impl FnOnce() {
    let state = self.shared_state.clone();
    let repo_clone = self.repo.clone();

    move || {
      state.set_loading(LoadingOperation::Deleting);
      state.send_render();

      let branches = state.get_branches();
      let selected_idx = state.get_selected_index();

      // Get branches staged for deletion
      let staged_branches: Vec<(usize, GitBranch)> = branches
        .iter()
        .enumerate()
        .filter(|(_, branch_item)| branch_item.staged_for_deletion)
        .map(|(idx, branch_item)| (idx, branch_item.branch.clone()))
        .collect();

      // Early return if nothing to delete
      if staged_branches.is_empty() {
        state.set_loading(LoadingOperation::None);
        state.send_render();
        return;
      }

      let future = async move {
        let mut indexes_to_delete: Vec<usize> = Vec::new();

        // Try to delete each branch
        for (branch_index, branch) in staged_branches {
          let del_result = repo_clone.delete_branch(&branch).await;
          if del_result.is_ok() {
            indexes_to_delete.push(branch_index);
          } else {
            // TODO: Track individual branch deletion errors
            if let Err(err) = del_result {
              error!("Failed to delete branch {}: {}", branch.name, err);
            }
          }
        }

        if indexes_to_delete.is_empty() {
          state.set_loading(LoadingOperation::None);
          state.send_render();
          return;
        }

        // Sort and reverse, so we remove branches starting from the end,
        // which means we don't need to worry about changing array positions.
        indexes_to_delete.sort();
        indexes_to_delete.reverse();

        let mut branches = state.get_branches();
        for index in indexes_to_delete {
          branches.remove(index);
        }

        // Adjust selected index
        let mut new_selected_idx = selected_idx;
        if new_selected_idx >= branches.len() && !branches.is_empty() {
          new_selected_idx = branches.len() - 1;
        } else if new_selected_idx != 0 && new_selected_idx > 0 {
          new_selected_idx -= 1;
        }

        state.update_branches(branches);
        state.update_selected_index(new_selected_idx);
        state.set_loading(LoadingOperation::None);
        state.send_render();
      };

      spawn(future);
    }
  }

  fn create_branch(&self, name: String) -> impl FnOnce() {
    let state = self.shared_state.clone();
    let repo_clone = self.repo.clone();

    move || {
      let branch = GitBranch { name: name.clone(), is_head: false, upstream: None };
      state.set_loading(LoadingOperation::Creating);
      state.send_render();

      let future = async move {
        // Create branch
        let create_result = repo_clone.create_branch(&branch).await;
        if let Err(err) = create_result {
          error!("{}", err);
          state.set_error(Some(err));
          state.set_loading(LoadingOperation::None);
          state.send_render();
          return;
        }

        // Checkout the new branch
        let checkout_result = repo_clone.checkout_branch_from_name(&name).await;
        if let Err(err) = checkout_result {
          error!("{}", err);
          state.set_error(Some(err));
          state.set_loading(LoadingOperation::None);
          state.send_render();
          return;
        }

        // Update branches
        let mut branches = state.get_branches();
        branches.push(BranchItem::new(branch, true));
        branches.sort_by(|a, b| a.branch.name.cmp(&b.branch.name));

        // Update head status
        for existing_branch in branches.iter_mut() {
          existing_branch.branch.is_head = existing_branch.branch.name == name;
        }

        // Find position of new branch
        let new_selected = branches.iter().position(|b| b.branch.name == name).unwrap_or(0);

        state.update_branches(branches);
        state.update_selected_index(new_selected);
        state.set_loading(LoadingOperation::None);
        state.send_render();
      };

      spawn(future);
    }
  }

  fn render_list(&mut self, f: &mut Frame<'_>, area: Rect) {
    // Sync state before rendering
    self.sync_state_for_render();

    // TODO don't clone, figure out the index to place the pseudo branch in the list
    let mut branches = self.branches.clone();
    let input_state = self.branch_input.input_state.clone();
    if input_state.value.is_some() && self.mode == Mode::Input {
      let content = input_state.value.unwrap();
      branches.push(BranchItem {
        branch: GitBranch::new(content.clone()),
        staged_for_creation: true,
        staged_for_deletion: false,
        is_valid_name: self.branch_input.input_state.is_valid.unwrap_or(false),
      });
      branches.sort_by(|a, b| a.branch.name.cmp(&b.branch.name));
      self.list_state.select(branches.iter().position(|bi| bi.staged_for_creation))
    } else {
      self.list_state.select(Some(self.selected_index));
    }

    let mut title = String::from("Local Branches");
    match self.loading {
      LoadingOperation::LoadingBranches => title = String::from("Loading Branches..."),
      LoadingOperation::CheckingOut => title = String::from("Checking Out Branch..."),
      LoadingOperation::Creating => title = String::from("Creating Branch..."),
      LoadingOperation::Deleting => title = String::from("Deleting Branch..."),
      LoadingOperation::None => {},
    }

    let render_items: Vec<ListItem> = branches.iter().map(|git_branch| git_branch.render()).collect();
    let list = List::new(render_items)
      .block(Block::default().title(title).borders(Borders::ALL))
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().add_modifier(Modifier::BOLD))
      .highlight_symbol("→")
      .repeat_highlight_symbol(true);

    f.render_stateful_widget(list, area, &mut self.list_state);
  }

  fn render_error(&mut self, f: &mut Frame<'_>, area: Rect) {
    if self.error.is_none() {
      return;
    }
    let error_message = self.error.as_ref().unwrap().clone();
    let text = Text::from(error_message);
    let component = Paragraph::new(text)
      .block(Block::bordered().title("Error"))
      .style(Style::from(Color::Red))
      .wrap(Wrap { trim: true });
    f.render_widget(component, area);
  }
}

#[async_trait::async_trait]
impl Component for BranchList {
  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
    // Sync with shared state before rendering
    self.sync_state_for_render();

    let chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([
        Constraint::Length(if self.error.is_some() { 3 } else { 0 }),
        Constraint::Min(1),
        Constraint::Length(if self.mode == Mode::Input { 3 } else { 0 }),
        Constraint::Length(3),
      ])
      .split(area);

    if self.error.is_some() {
      self.render_error(f, chunks[0]);
    }

    self.render_list(f, chunks[1]);

    if self.mode == Mode::Input {
      self.branch_input.render(f, chunks[2]);
    }

    self.instruction_footer.render(f, chunks[3], &self.branches, self.get_selected_branch());

    Ok(())
  }

  async fn handle_key_events(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
    self.clear_error();

    if self.mode == Mode::Input {
      return Ok(Some(Action::UpdateNewBranchName(key)));
    }

    let action = match key {
      KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Some(Action::SelectNextBranch)
      },
      KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Some(Action::SelectPreviousBranch)
      },
      KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::SHIFT, kind: _, state: _ } => {
        Some(Action::InitNewBranch)
      },
      KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Some(Action::CheckoutSelectedBranch)
      },
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::SHIFT, kind: _, state: _ } => {
        Some(Action::UnstageBranchForDeletion)
      },
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, kind: _, state: _ } => {
        Some(Action::DeleteStagedBranches)
      },
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        if self.get_selected_branch().is_none() {
          None
        } else {
          let selected = self.get_selected_branch().unwrap();
          if selected.staged_for_deletion { Some(Action::DeleteBranch) } else { Some(Action::StageBranchForDeletion) }
        }
      },
      _ => None,
    };

    Ok(action)
  }

  async fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
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
        info!("BranchList: Initializing new branch input");
        self.mode = Mode::Input;
        self.branch_input.init_style();
        Ok(Some(Action::StartInputMode))
      },
      Action::EndInputMod => {
        self.mode = Mode::Selection;
        Ok(None)
      },
      Action::UpdateNewBranchName(key_event) => {
        let branches = self.shared_state.get_branches();
        let branch_refs: Vec<&GitBranch> = branches.iter().map(|branch_item| &branch_item.branch).collect();

        // Still awaiting this one because it's UI-related and needs to be synchronous
        let action = self.branch_input.handle_key_event(key_event, &*self.repo, branch_refs).await;

        Ok(action)
      },
      Action::CheckoutSelectedBranch => {
        info!("BranchList: Checking out selected branch");
        let operation = self.checkout_selected();
        operation();
        Ok(None)
      },
      Action::CreateBranch(name) => {
        info!("BranchList: Creating branch '{}'", name);
        self.mode = Mode::Selection;
        let operation = self.create_branch(name);
        operation();
        Ok(Some(Action::EndInputMod))
      },
      Action::StageBranchForDeletion => {
        info!("BranchList: Staging branch for deletion");
        self.stage_selected_for_deletion(true);
        Ok(None)
      },
      Action::UnstageBranchForDeletion => {
        info!("BranchList: Unstaging branch from deletion");
        self.stage_selected_for_deletion(false);
        Ok(None)
      },
      Action::DeleteBranch => {
        info!("BranchList: Deleting selected branch");
        let operation = self.deleted_selected();
        operation();
        Ok(None)
      },
      Action::DeleteStagedBranches => {
        info!("BranchList: Deleting staged branches");
        let operation = self.delete_staged_branches();
        operation();
        Ok(None)
      },
      Action::Refresh => {
        let operation = self.load_branches();
        operation();
        Ok(None)
      },
      _ => Ok(None),
    }
  }
}
