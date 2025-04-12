use std::{
  sync::{Arc, Mutex},
  time::SystemTime,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
  Frame,
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  widgets::{Block, Borders, List, ListItem, ListState},
};
use tokio::{sync::mpsc::UnboundedSender, task::spawn};
use tracing::{error, info, warn};

use super::{InstructionFooter, StashInput, StashItem};
use crate::{
  action::Action,
  components::{AsyncComponent, Component},
  git::types::{GitRepo, GitStash},
};

#[derive(Debug, Clone, PartialEq, Eq)]

enum Mode {
  Selection,
  Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadingOperation {
  None,
  LoadingStashes(SystemTime),
  Applying(SystemTime),
  Popping(SystemTime),
  Dropping(SystemTime),
  DroppingWithProgress(SystemTime, usize, usize), // (time, current, total)
  Stashing(SystemTime),
}

// Shared state that can be accessed from async blocks
#[derive(Clone)]
struct SharedState {
  loading: Arc<Mutex<LoadingOperation>>,
  stashes: Arc<Mutex<Vec<StashItem>>>,
  selected_index: Arc<Mutex<usize>>,
  action_tx: Arc<Mutex<Option<UnboundedSender<Action>>>>,
}

impl SharedState {
  fn new() -> Self {
    SharedState {
      loading: Arc::new(Mutex::new(LoadingOperation::None)),
      stashes: Arc::new(Mutex::new(Vec::new())),
      selected_index: Arc::new(Mutex::new(0)),
      action_tx: Arc::new(Mutex::new(None)),
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

  fn send_error(&self, message: String) {
    let action_tx = self.action_tx.lock().unwrap();
    if action_tx.is_some() {
      let _ = action_tx.as_ref().unwrap().send(Action::Error(message));
    }
  }

  fn update_stashes(&self, new_stashes: Vec<StashItem>) {
    let mut stashes_guard = self.stashes.lock().unwrap();
    *stashes_guard = new_stashes;
  }

  fn get_stashes(&self) -> Vec<StashItem> {
    self.stashes.lock().unwrap().clone()
  }

  fn get_selected_index(&self) -> usize {
    *self.selected_index.lock().unwrap()
  }

  fn set_selected_index(&self, index: usize) {
    let mut selected_index_guard = self.selected_index.lock().unwrap();
    *selected_index_guard = index;
  }
}

pub struct StashList {
  repo: Arc<dyn GitRepo>,
  shared_state: SharedState,
  mode: Mode,
  list_state: ListState,
  instruction_footer: InstructionFooter,
  stash_input: StashInput, // Add stash input component
}

impl StashList {
  pub fn new(repo: Arc<dyn GitRepo>) -> Self {
    let shared_state = SharedState::new();

    StashList {
      repo,
      shared_state,
      mode: Mode::Selection,
      list_state: ListState::default(),
      instruction_footer: InstructionFooter::default(),
      stash_input: StashInput::new(), // Initialize stash input
    }
  }

  pub fn load_stashes(&self) -> impl FnOnce() {
    let state = self.shared_state.clone();
    let repo_clone = self.repo.clone();

    move || {
      state.set_loading(LoadingOperation::LoadingStashes(SystemTime::now()));
      state.send_render();

      let future = async move {
        let stashes_result = repo_clone.stashes().await;

        match stashes_result {
          Ok(stashes) => {
            let stash_items = stashes.iter().map(|stash| StashItem::new(stash.clone())).collect();
            state.update_stashes(stash_items);
            state.set_loading(LoadingOperation::None);
            state.send_render();
          },
          Err(err) => {
            error!("{}", err);
            state.send_error(err.to_string());
            state.set_loading(LoadingOperation::None);
            state.send_render();
          },
        }
      };

      spawn(future);
    }
  }

  pub fn select_previous(&mut self) {
    let stashes = self.shared_state.stashes.lock().unwrap();
    let mut selected_idx = self.shared_state.selected_index.lock().unwrap();

    if *selected_idx == 0 {
      *selected_idx = stashes.len() - 1;
      return;
    }
    if *selected_idx >= stashes.len() {
      *selected_idx = stashes.len() - 1;
      return;
    }
    *selected_idx -= 1;
  }

  pub fn select_next(&mut self) {
    let stashes = self.shared_state.stashes.lock().unwrap();
    let mut selected_idx = self.shared_state.selected_index.lock().unwrap();

    if *selected_idx == stashes.len() - 1 {
      *selected_idx = 0;
      return;
    }
    if *selected_idx >= stashes.len() {
      *selected_idx = 0;
      return;
    }
    *selected_idx += 1;
  }

  fn get_selected_stash(&self) -> Option<StashItem> {
    let stashes = self.shared_state.get_stashes();
    if stashes.is_empty() {
      return None;
    }
    let selected_index = self.shared_state.get_selected_index();
    if selected_index >= stashes.len() {
      return None;
    }
    stashes.get(selected_index).cloned()
  }

  fn apply_selected(&self) -> impl FnOnce() {
    let state = self.shared_state.clone();
    let repo_clone = self.repo.clone();

    move || {
      let stashes = state.get_stashes();
      let selected_idx = state.get_selected_index();

      let maybe_selected = stashes.get(selected_idx);
      if maybe_selected.is_none() {
        return;
      }

      let stash_to_apply = maybe_selected.unwrap().stash.clone();
      state.set_loading(LoadingOperation::Applying(SystemTime::now()));
      state.send_render();

      let future = async move {
        let apply_result = repo_clone.apply_stash(&stash_to_apply).await;

        if let Err(err) = apply_result {
          error!("{}", err);
          state.send_error(err.to_string());
          state.set_loading(LoadingOperation::None);
          state.send_render();
          return;
        }

        // Refresh stashes after applying
        let stashes_result = repo_clone.stashes().await;
        if let Ok(stashes) = stashes_result {
          let stash_items = stashes.iter().map(|stash| StashItem::new(stash.clone())).collect();
          state.update_stashes(stash_items);
        }

        state.set_loading(LoadingOperation::None);
        state.send_render();
      };

      spawn(future);
    }
  }

  fn pop_selected(&self) -> impl FnOnce() {
    let state = self.shared_state.clone();
    let repo_clone = self.repo.clone();

    move || {
      let stashes = state.get_stashes();
      let selected_idx = state.get_selected_index();

      let maybe_selected = stashes.get(selected_idx);
      if maybe_selected.is_none() {
        return;
      }

      let stash_to_pop = maybe_selected.unwrap().stash.clone();
      state.set_loading(LoadingOperation::Popping(SystemTime::now()));
      state.send_render();

      let future = async move {
        let pop_result = repo_clone.pop_stash(&stash_to_pop).await;

        if let Err(err) = pop_result {
          error!("{}", err);
          state.send_error(err.to_string());
          state.set_loading(LoadingOperation::None);
          state.send_render();
          return;
        }

        // Refresh stashes after popping
        let stashes_result = repo_clone.stashes().await;
        if let Ok(stashes) = stashes_result {
          let stash_items =
            stashes.iter().filter(|stash| **stash != stash_to_pop).map(|stash| StashItem::new(stash.clone())).collect();
          state.update_stashes(stash_items);
        }

        state.set_loading(LoadingOperation::None);
        state.send_render();
      };

      spawn(future);
    }
  }

  fn drop_selected(&self) -> impl FnOnce() {
    let state = self.shared_state.clone();
    let repo_clone = self.repo.clone();

    move || {
      let stashes = state.get_stashes();
      let selected_idx = state.get_selected_index();

      let maybe_selected = stashes.get(selected_idx);
      if maybe_selected.is_none() {
        return;
      }

      let stash_to_drop = maybe_selected.unwrap().stash.clone();
      state.set_loading(LoadingOperation::Dropping(SystemTime::now()));
      state.send_render();

      let future = async move {
        let drop_result = repo_clone.drop_stash(&stash_to_drop).await;

        if let Err(err) = drop_result {
          error!("{}", err);
          state.send_error(err.to_string());
          state.set_loading(LoadingOperation::None);
          state.send_render();
          return;
        }

        // Refresh stashes after dropping
        let stashes_result = repo_clone.stashes().await;
        if let Ok(stashes) = stashes_result {
          let stash_items = stashes.iter().map(|stash| StashItem::new(stash.clone())).collect();
          state.update_stashes(stash_items);
          // Adjust selected index if it's beyond bounds
          let mut new_selected_idx = selected_idx;
          if new_selected_idx >= stashes.len() && !stashes.is_empty() {
            new_selected_idx -= 1;
          }
          state.set_selected_index(new_selected_idx);
        }

        state.set_loading(LoadingOperation::None);
        state.send_render();
      };

      spawn(future);
    }
  }

  pub fn stage_selected_for_deletion(&mut self, stage: bool) {
    let selected_idx = self.shared_state.get_selected_index();
    let mut stashes = self.shared_state.get_stashes();

    let maybe_selected = stashes.get_mut(selected_idx);
    if maybe_selected.is_none() {
      return;
    }

    let selected = maybe_selected.unwrap();
    selected.stage_for_deletion(stage);
    self.shared_state.update_stashes(stashes);
  }

  pub fn delete_staged_stashes(&self) -> impl FnOnce() {
    let state = self.shared_state.clone();
    let repo_clone = self.repo.clone();

    move || {
      let stashes = state.get_stashes();

      // Get stashes staged for deletion, sorted by index in descending order
      let mut staged_stashes: Vec<(usize, GitStash)> = stashes
        .iter()
        .enumerate()
        .filter(|(_, stash_item)| stash_item.staged_for_deletion)
        .map(|(idx, stash_item)| (idx, stash_item.stash.clone()))
        .collect();

      // Sort by index in descending order so we delete from highest to lowest
      staged_stashes.sort_by(|a, b| b.0.cmp(&a.0));

      // Early return if nothing to delete
      if staged_stashes.is_empty() {
        state.set_loading(LoadingOperation::None);
        state.send_render();
        return;
      }

      let total_stashes = staged_stashes.len();
      let start_time = SystemTime::now();
      state.set_loading(LoadingOperation::DroppingWithProgress(start_time, 0, total_stashes));
      state.send_render();

      let future = async move {
        let mut deleted_count = 0;
        let mut indexes_to_delete: Vec<usize> = Vec::new();

        // Try to delete each stash in reverse order
        for (i, (stash_index, stash)) in staged_stashes.into_iter().enumerate() {
          let del_result = repo_clone.drop_stash(&stash).await;
          if del_result.is_ok() {
            deleted_count += 1;
            indexes_to_delete.push(stash_index);
          } else if let Err(err) = del_result {
            error!("Failed to delete stash {}: {}", stash.stash_id, err);
          }
          state.set_loading(LoadingOperation::DroppingWithProgress(start_time, i + 1, total_stashes));
          state.send_render();
        }

        if deleted_count == 0 {
          state.set_loading(LoadingOperation::None);
          state.send_render();
          return;
        }

        // Sort and reverse, so we remove stashes starting from the end,
        // which means we don't need to worry about changing array positions.
        indexes_to_delete.sort();
        indexes_to_delete.reverse();

        // Refresh stashes after all deletions are complete
        if let Ok(stashes) = repo_clone.stashes().await {
          let stash_items = stashes.iter().map(|stash| StashItem::new(stash.clone())).collect();
          state.update_stashes(stash_items);
          // Adjust selected index to the smallest deleted index
          let new_selected_idx = indexes_to_delete.last().unwrap_or(&0);
          state.set_selected_index(*new_selected_idx);
        }

        state.set_loading(LoadingOperation::None);
        state.send_render();
      };

      spawn(future);
    }
  }

  fn create_stash(&self, message: String) -> impl FnOnce() {
    let state = self.shared_state.clone();
    let repo_clone = self.repo.clone();

    move || {
      state.set_loading(LoadingOperation::Stashing(SystemTime::now()));
      state.send_render();

      let future = async move {
        let stash_result = repo_clone.stash_with_message(&message).await;

        if let Err(err) = stash_result {
          error!("{}", err);
          state.send_error(err.to_string());
          state.set_loading(LoadingOperation::None);
          state.send_render();
          return;
        }

        if let Ok(did_stash) = stash_result {
          if !did_stash {
            state.send_error("No local changes to stash".to_string());
            state.set_loading(LoadingOperation::None);
            state.send_render();
            return;
          }
        }

        // Refresh stashes after creating
        let stashes_result = repo_clone.stashes().await;
        if let Ok(stashes) = stashes_result {
          let stash_items = stashes.iter().map(|stash| StashItem::new(stash.clone())).collect();
          state.update_stashes(stash_items);
        }

        state.set_loading(LoadingOperation::None);
        state.send_render();
      };

      spawn(future);
    }
  }

  fn render_list(&mut self, f: &mut Frame<'_>, area: Rect) {
    let mut title = String::from("Stashes");

    // Extract state for rendering
    let loading = *self.shared_state.loading.lock().unwrap();
    let stashes = self.shared_state.get_stashes();
    let selected_index = self.shared_state.get_selected_index();

    match loading {
      LoadingOperation::LoadingStashes(time) => title = format!("Loading Stashes...({})", format_time_elapsed(time)),
      LoadingOperation::Applying(time) => title = format!("Applying Stash...({})", format_time_elapsed(time)),
      LoadingOperation::Popping(time) => title = format!("Popping Stash...({})", format_time_elapsed(time)),
      LoadingOperation::Dropping(time) => title = format!("Dropping Stash...({})", format_time_elapsed(time)),
      LoadingOperation::DroppingWithProgress(time, current, total) => {
        title = format!("Dropping Stash {}/{}...({})", current, total, format_time_elapsed(time))
      },
      LoadingOperation::Stashing(time) => title = format!("Stashing...({})", format_time_elapsed(time)),
      LoadingOperation::None => {},
    }

    let render_items: Vec<ListItem> = stashes.iter().map(|stash| stash.render()).collect();
    let list = List::new(render_items)
      .block(Block::default().title(title).borders(Borders::ALL))
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().add_modifier(Modifier::BOLD))
      .highlight_symbol("→")
      .repeat_highlight_symbol(true);

    self.list_state.select(Some(selected_index));
    f.render_stateful_widget(list, area, &mut self.list_state);
  }
}

impl Component for StashList {
  fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
    *self.shared_state.action_tx.lock().unwrap() = Some(tx);
    self.stash_input.init_style(); // Ensure style is initialized
    Ok(())
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
    // Extract state for rendering
    let stashes = self.shared_state.get_stashes();
    let selected_index = self.shared_state.get_selected_index();

    let layout_base = Layout::default().direction(Direction::Vertical);
    let chunks = layout_base
      .constraints([
        Constraint::Min(1),
        Constraint::Length(if self.mode == Mode::Input { 3 } else { 0 }),
        Constraint::Length(3),
      ])
      .split(area);

    let selected_stash = stashes.get(selected_index);
    let has_staged_stashes = stashes.iter().any(|s| s.staged_for_deletion); // Calculate if any stashes are staged
    self.render_list(frame, chunks[0]);

    if self.mode == Mode::Input {
      self.stash_input.render(frame, chunks[1]);
    }

    self.instruction_footer.render(frame, chunks[2], selected_stash, has_staged_stashes); // Pass the new argument

    Ok(())
  }
} // Correctly close the Component impl block

#[async_trait::async_trait]
impl AsyncComponent for StashList {
  async fn handle_events(&mut self, event: Option<crate::tui::Event>) -> color_eyre::Result<Option<Action>> {
    match event {
      Some(crate::tui::Event::Key(key)) => self.handle_key_events(key).await,
      _ => Ok(None),
    }
  }

  async fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
    match action {
      Action::SelectPreviousStash => {
        self.select_previous();
        Ok(None)
      },
      Action::SelectNextStash => {
        self.select_next();
        Ok(None)
      },
      Action::InitNewStash => {
        info!("StashList: Opening stash input");
        self.mode = Mode::Input;
        self.stash_input.init_style();
        Ok(Some(Action::StartInputMode))
      },
      Action::EndInputMod => {
        self.mode = Mode::Selection;
        Ok(None)
      },
      Action::ApplySelectedStash => {
        info!("StashList: Applying selected stash");
        let operation = self.apply_selected();
        operation();
        Ok(None)
      },
      Action::PopSelectedStash => {
        info!("StashList: Popping selected stash");
        let operation = self.pop_selected();
        operation();
        Ok(None)
      },
      Action::DropSelectedStash => {
        info!("StashList: Dropping selected stash");
        let operation = self.drop_selected();
        operation();
        Ok(None)
      },
      Action::StageStashForDeletion => {
        info!("StashList: Staging stash for deletion");
        self.stage_selected_for_deletion(true);
        Ok(None)
      },
      Action::UnstageStashForDeletion => {
        info!("StashList: Unstaging stash from deletion");
        self.stage_selected_for_deletion(false);
        Ok(None)
      },
      Action::DeleteStagedStashes => {
        info!("StashList: Deleting staged stashes");
        let operation = self.delete_staged_stashes();
        operation();
        Ok(None)
      },
      Action::Refresh => {
        let operation = self.load_stashes();
        operation();
        Ok(None)
      },
      Action::CreateStash(message) => {
        info!("StashList: Creating stash with message: {}", message);
        let operation = self.create_stash(message);
        operation();
        Ok(Some(Action::EndInputMod)) // End input mode after creating stash
      },
      _ => Ok(None),
    }
  }
}

impl StashList {
  // Add method to handle key events specifically for the input mode
  pub fn handle_input_key_event(&mut self, key: KeyEvent) -> Option<Action> {
    self.stash_input.handle_key_event(key)
  }

  async fn handle_key_events(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
    if self.mode == Mode::Input {
      return Ok(self.stash_input.handle_key_event(key));
    }
    let action = match key {
      KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Some(Action::SelectNextStash)
      },
      KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Some(Action::SelectPreviousStash)
      },
      KeyEvent { code: KeyCode::Char('a' | 'A'), modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Some(Action::ApplySelectedStash)
      },
      KeyEvent { code: KeyCode::Char('s' | 'S'), modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Some(Action::InitNewStash)
      },
      KeyEvent { code: KeyCode::Char('p' | 'P'), modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Some(Action::PopSelectedStash)
      },
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::SHIFT, kind: _, state: _ } => {
        Some(Action::UnstageStashForDeletion)
      },
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, kind: _, state: _ } => {
        Some(Action::DeleteStagedStashes)
      },
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        if self.get_selected_stash().is_none() {
          None
        } else {
          let selected = self.get_selected_stash().unwrap();
          if selected.staged_for_deletion {
            Some(Action::DropSelectedStash)
          } else {
            Some(Action::StageStashForDeletion)
          }
        }
      },
      _ => None,
    };

    Ok(action)
  }
}

// Move helper function outside impl blocks
fn format_time_elapsed(time: SystemTime) -> String {
  match time.elapsed() {
    Ok(elapsed) => format!("{:.1}s", elapsed.as_secs_f64()),
    Err(err) => {
      warn!("Failed to get system time {}", err);
      String::from("xs")
    },
  }
}

#[cfg(test)]
mod tests {
  use tokio::sync::mpsc;

  use super::*;
  use crate::git::types::MockGitRepo;

  #[tokio::test]
  async fn test_shared_state_loading() {
    let state = SharedState::new();
    state.set_loading(LoadingOperation::LoadingStashes(SystemTime::now()));

    let loading = *state.loading.lock().unwrap();
    assert!(matches!(loading, LoadingOperation::LoadingStashes(_)));
  }

  #[tokio::test]
  async fn test_shared_state_send_error() {
    let state = SharedState::new();
    let (tx, mut rx) = mpsc::unbounded_channel();
    *state.action_tx.lock().unwrap() = Some(tx);

    state.send_error("Test error".to_string());

    if let Some(Action::Error(message)) = rx.recv().await {
      assert_eq!(message, "Test error");
    } else {
      panic!("Expected Action::Error");
    }
  }

  #[tokio::test]
  async fn test_stash_list_select_previous() {
    let repo = Arc::new(MockGitRepo);
    let mut stash_list = StashList::new(repo);

    stash_list.shared_state.update_stashes(vec![
      StashItem::new(GitStash::new(0, "message1".to_string(), "stash@{0}".to_string(), "branch_name".to_string())),
      StashItem::new(GitStash::new(1, "message2".to_string(), "stash@{1}".to_string(), "branch_name".to_string())),
    ]);
    stash_list.shared_state.set_selected_index(0);

    stash_list.select_previous();
    assert_eq!(stash_list.shared_state.get_selected_index(), 1);
  }

  #[tokio::test]
  async fn test_stash_list_select_next() {
    let repo = Arc::new(MockGitRepo);
    let mut stash_list = StashList::new(repo);

    stash_list.shared_state.update_stashes(vec![
      StashItem::new(GitStash::new(0, "message1".to_string(), "stash@{0}".to_string(), "branch_name".to_string())),
      StashItem::new(GitStash::new(1, "message2".to_string(), "stash@{1}".to_string(), "branch_name".to_string())),
    ]);
    stash_list.shared_state.set_selected_index(0);

    stash_list.select_next();
    assert_eq!(stash_list.shared_state.get_selected_index(), 1);
  }

  #[tokio::test]
  async fn test_get_selected_stash() {
    let repo = Arc::new(MockGitRepo);
    let stash_list = StashList::new(repo);

    stash_list.shared_state.update_stashes(vec![StashItem::new(GitStash::new(
      0,
      "message1".to_string(),
      "stash@{0}".to_string(),
      "branch_name".to_string(),
    ))]);
    stash_list.shared_state.set_selected_index(0);

    let selected_stash = stash_list.get_selected_stash();

    assert!(selected_stash.is_some());
    assert_eq!(selected_stash.unwrap().stash.message, "message1");
  }

  #[tokio::test]
  async fn test_stage_selected_for_deletion() {
    let repo = Arc::new(MockGitRepo);
    let mut stash_list = StashList::new(repo);

    stash_list.shared_state.update_stashes(vec![StashItem::new(GitStash::new(
      0,
      "message1".to_string(),
      "stash@{0}".to_string(),
      "branch_name".to_string(),
    ))]);
    stash_list.shared_state.set_selected_index(0);

    stash_list.stage_selected_for_deletion(true);
    let stashes = stash_list.shared_state.get_stashes();

    assert!(stashes[0].staged_for_deletion);
  }

  mod key_event_tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_key_events() {
      let repo = Arc::new(MockGitRepo);
      let mut stash_list = StashList::new(repo);

      let key_event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
      let action = stash_list.handle_key_events(key_event).await.unwrap();

      assert_eq!(action, Some(Action::SelectNextStash));
    }

    #[tokio::test]
    async fn test_handle_key_event_up() {
      let repo = Arc::new(MockGitRepo);
      let mut stash_list = StashList::new(repo);

      let key_event = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
      let action = stash_list.handle_key_events(key_event).await.unwrap();

      assert_eq!(action, Some(Action::SelectPreviousStash));
    }

    #[tokio::test]
    async fn test_handle_key_event_apply() {
      let repo = Arc::new(MockGitRepo);
      let mut stash_list = StashList::new(repo);

      let key_event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
      let action = stash_list.handle_key_events(key_event).await.unwrap();

      assert_eq!(action, Some(Action::ApplySelectedStash));
    }

    #[tokio::test]
    async fn test_handle_key_event_init_new_stash() {
      let repo = Arc::new(MockGitRepo);
      let mut stash_list = StashList::new(repo);

      let key_event = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
      let action = stash_list.handle_key_events(key_event).await.unwrap();

      assert_eq!(action, Some(Action::InitNewStash));
    }

    #[tokio::test]
    async fn test_handle_key_event_pop() {
      let repo = Arc::new(MockGitRepo);
      let mut stash_list = StashList::new(repo);

      let key_event = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
      let action = stash_list.handle_key_events(key_event).await.unwrap();

      assert_eq!(action, Some(Action::PopSelectedStash));
    }

    #[tokio::test]
    async fn test_handle_key_event_unstage() {
      let repo = Arc::new(MockGitRepo);
      let mut stash_list = StashList::new(repo);

      let key_event = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::SHIFT);
      let action = stash_list.handle_key_events(key_event).await.unwrap();

      assert_eq!(action, Some(Action::UnstageStashForDeletion));
    }

    #[tokio::test]
    async fn test_handle_key_event_delete_staged() {
      let repo = Arc::new(MockGitRepo);
      let mut stash_list = StashList::new(repo);

      let key_event = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
      let action = stash_list.handle_key_events(key_event).await.unwrap();

      assert_eq!(action, Some(Action::DeleteStagedStashes));
    }

    #[tokio::test]
    async fn test_handle_key_event_stage_or_drop() {
      let repo = Arc::new(MockGitRepo);
      let mut stash_list = StashList::new(repo);

      stash_list.shared_state.update_stashes(vec![StashItem::new(GitStash::new(
        0,
        "message1".to_string(),
        "stash@{0}".to_string(),
        "branch_name".to_string(),
      ))]);
      stash_list.shared_state.set_selected_index(0);

      let key_event = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
      let action = stash_list.handle_key_events(key_event).await.unwrap();

      assert_eq!(action, Some(Action::StageStashForDeletion));
    }

    #[tokio::test]
    async fn test_handle_key_event_drop_selected_stash() {
      let repo = Arc::new(MockGitRepo);
      let mut stash_list = StashList::new(repo);

      stash_list.shared_state.update_stashes(vec![StashItem::new(GitStash::new(
        0,
        "message1".to_string(),
        "stash@{0}".to_string(),
        "branch_name".to_string(),
      ))]);
      stash_list.shared_state.set_selected_index(0);

      let key_event = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
      stash_list.stage_selected_for_deletion(true); // Stage for deletion first
      let action = stash_list.handle_key_events(key_event).await.unwrap();

      assert_eq!(action, Some(Action::DropSelectedStash));
    }

    #[tokio::test]
    async fn test_handle_key_event_stage_stash_for_deletion() {
      let repo = Arc::new(MockGitRepo);
      let mut stash_list = StashList::new(repo);

      stash_list.shared_state.update_stashes(vec![StashItem::new(GitStash::new(
        0,
        "message1".to_string(),
        "stash@{0}".to_string(),
        "branch_name".to_string(),
      ))]);
      stash_list.shared_state.set_selected_index(0);

      let key_event = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
      let action = stash_list.handle_key_events(key_event).await.unwrap();

      assert_eq!(action, Some(Action::StageStashForDeletion));
    }
  }
}
