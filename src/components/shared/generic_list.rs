// src/components/shared/generic_list.rs

use std::{
  fmt::Debug,
  sync::{Arc, Mutex}, // Keep Mutex for selected_index and items for now
  time::SystemTime,
};

use async_trait::async_trait;
use color_eyre::Result;
use crossterm::event::KeyCode; // Import KeyCode
use crossterm::event::KeyEvent;
use ratatui::{
  Frame as TuiFrame, // Alias to avoid conflict with crate::tui::Frame
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  widgets::{Block, Borders, List, ListItem, ListState},
};
use tokio::{sync::mpsc::UnboundedSender, task::spawn};
use tracing::{error, info, warn};

use super::generic_input::GenericInputComponent;
use crate::{
  action::Action,
  components::{
    AsyncComponent, Component,
    traits::{
      input_handler::InputHandler, list_action_handler::ListActionHandler, list_data_source::ListDataSource,
      list_item_wrapper::ListItemWrapper, managed_item::ManagedItem,
    },
  },
  git::types::GitRepo,
  tui::Frame, // Use our Frame type alias
};

// --- Enums (Common) ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
  Selection,
  Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingOperation {
  None,
  Loading(SystemTime),
  Processing(SystemTime),                           // Generic processing state
  ProcessingWithProgress(SystemTime, usize, usize), // (time, current, total)
}

// --- Shared State (Simplified) ---
// We'll manage most state directly in the component, but keep items/index shared for now
// as async operations might need to update them directly or indirectly.
// Consider passing action_tx to async ops instead for cleaner state updates.
#[derive(Clone)]
struct SharedListState<W: Clone + Send + Sync + 'static, T: ManagedItem> {
  // Added T bound here
  items: Arc<Mutex<Vec<W>>>,
  selected_index: Arc<Mutex<usize>>,
  _phantom_t: std::marker::PhantomData<T>, // Phantom data for T
}

impl<W: Clone + Send + Sync + 'static, T: ManagedItem> SharedListState<W, T> {
  // Added T bound here
  fn new() -> Self {
    Self {
      items: Arc::new(Mutex::new(Vec::new())),
      selected_index: Arc::new(Mutex::new(0)),
      _phantom_t: std::marker::PhantomData,
    }
  }

  fn get_items(&self) -> Vec<W> {
    self.items.lock().unwrap().clone()
  }

  fn get_selected_index(&self) -> usize {
    *self.selected_index.lock().unwrap()
  }

  fn update_items(&self, new_items: Vec<W>) {
    *self.items.lock().unwrap() = new_items;
  }

  fn update_selected_index(&self, index: usize) {
    *self.selected_index.lock().unwrap() = index;
  }

  fn get_item_at_index(&self, index: usize) -> Option<W> {
    self.items.lock().unwrap().get(index).cloned()
  }

  fn get_selected_item(&self) -> Option<W> {
    let index = self.get_selected_index();
    self.get_item_at_index(index)
  }

  fn get_items_count(&self) -> usize {
    self.items.lock().unwrap().len()
  }

  fn stage_item_for_deletion(&self, index: usize, stage: bool) -> bool
  where
    W: ListItemWrapper<T>,
  {
    // Removed T bound here, already on impl
    let mut items_guard = self.items.lock().unwrap();
    if let Some(item) = items_guard.get_mut(index) {
      // Add safety check if needed (e.g., cannot stage HEAD branch)
      // This logic might be better placed within the action handler or component update
      item.stage_for_deletion(stage);
      true
    } else {
      false
    }
  }

  fn get_staged_for_deletion(&self) -> Vec<W>
  where
    W: ListItemWrapper<T>,
  {
    // Removed T bound here
    self.items.lock().unwrap().iter().filter(|item| item.is_staged_for_deletion()).cloned().collect()
  }

  fn has_staged_items(&self) -> bool
  where
    W: ListItemWrapper<T>,
  {
    // Removed T bound here
    self.items.lock().unwrap().iter().any(|item| item.is_staged_for_deletion())
  }

  fn remove_item_at_index(&self, index: usize) -> Option<W> {
    let mut items_guard = self.items.lock().unwrap();
    if index < items_guard.len() { Some(items_guard.remove(index)) } else { None }
  }
}

// --- Generic List Component ---

pub struct GenericListComponent<W, T, DS, AH, IH>
where
  W: ListItemWrapper<T> + Debug + Clone + Send + Sync + 'static, // Ensure W is fully bounded
  T: ManagedItem + Debug + Clone + Send + Sync + 'static,        // Add Clone + Send + Sync + 'static
  DS: ListDataSource<T> + Send + Sync + 'static,                 // Add Send + Sync + 'static
  AH: ListActionHandler<W, T> + Send + Sync + 'static,           // Add Send + Sync + 'static
  IH: InputHandler<T> + Send + Sync + 'static,                   // Add Send + Sync + 'static
{
  repo: Arc<dyn GitRepo>,
  data_source: Arc<DS>,
  action_handler: Arc<AH>,
  // input_handler: Arc<IH>, // Input handler is owned by input_component

  // State
  mode: Mode,
  loading: LoadingOperation,
  shared_state: SharedListState<W, T>, // Holds items and selected_index
  list_state: ListState,               // Ratatui's list state

  // Sub-components
  input_component: GenericInputComponent<IH, T>,
  // instruction_footer: InstructionFooter, // TODO: Refactor footer later

  // Communication
  action_tx: Option<UnboundedSender<Action>>,

  // Type markers
  _phantom_t: std::marker::PhantomData<T>, /* Already present
                                            * _phantom_w: std::marker::PhantomData<W>, // Removed, W is used in SharedListState */
}

impl<W, T, DS, AH, IH> GenericListComponent<W, T, DS, AH, IH>
where
  W: ListItemWrapper<T> + Debug + Clone + Send + Sync + 'static, // Ensure W is fully bounded
  T: ManagedItem + Debug + Clone + Send + Sync + 'static,
  DS: ListDataSource<T> + Default + Send + Sync + 'static, // Add Default + Send + Sync + 'static
  AH: ListActionHandler<W, T> + Default + Send + Sync + 'static, // Add Default + Send + Sync + 'static
  IH: InputHandler<T> + Default + Send + Sync + 'static,   // Add Default + Send + Sync + 'static
{
  pub fn new(repo: Arc<dyn GitRepo>) -> Self {
    let data_source = Arc::new(DS::default());
    let action_handler = Arc::new(AH::default());
    let input_handler = Arc::new(IH::default());
    let shared_state: SharedListState<W, T> = SharedListState::new();

    // Create Arc<Vec<T>> from initial empty items for input component
    // Need to get T items from W items in shared_state if not empty initially (it is empty here)
    let initial_items_t = Arc::new(Vec::<T>::new());

    let input_component = GenericInputComponent::new(input_handler.clone(), repo.clone(), initial_items_t);

    Self {
      repo,
      data_source,
      action_handler,
      // input_handler, // Removed, owned by input_component
      mode: Mode::Selection,
      loading: LoadingOperation::None,
      shared_state,
      list_state: ListState::default(),
      input_component,
      // instruction_footer: InstructionFooter::default(), // TODO
      action_tx: None,
      _phantom_t: std::marker::PhantomData,
      // _phantom_w: std::marker::PhantomData, // Removed
    }
  }

  fn send_action(&self, action: Action) {
    if let Some(tx) = &self.action_tx {
      if let Err(e) = tx.send(action) {
        error!("Failed to send action: {}", e);
      }
    }
  }

  // --- State Management ---

  fn sync_state_for_render(&mut self) {
    // No longer needed if state is managed directly or updated via actions
  }

  fn set_loading(&mut self, op: LoadingOperation) {
    self.loading = op;
    self.send_action(Action::Render); // Trigger re-render when loading state changes
  }

  fn select_next(&mut self) {
    let count = self.shared_state.get_items_count();
    if count == 0 {
      return;
    }
    let current_index = self.shared_state.get_selected_index();
    let next_index = if current_index >= count - 1 { 0 } else { current_index + 1 };
    self.shared_state.update_selected_index(next_index);
    self.list_state.select(Some(next_index)); // Update ratatui state
  }

  fn select_previous(&mut self) {
    let count = self.shared_state.get_items_count();
    if count == 0 {
      return;
    }
    let current_index = self.shared_state.get_selected_index();
    let prev_index = if current_index == 0 { count - 1 } else { current_index - 1 };
    self.shared_state.update_selected_index(prev_index);
    self.list_state.select(Some(prev_index)); // Update ratatui state
  }

  fn get_selected_item_wrapper(&self) -> Option<W> {
    self.shared_state.get_selected_item()
  }

  // --- Async Operations ---

  fn load_items(&mut self) {
    self.set_loading(LoadingOperation::Loading(SystemTime::now()));
    let tx = self.action_tx.clone();
    let ds = self.data_source.clone();
    let repo_clone = self.repo.clone();
    let shared_state_clone = self.shared_state.clone();

    spawn(async move {
      match ds.fetch_items(repo_clone).await {
        Ok(items_t) => {
          // Convert T to W using ListItemWrapper::new
          let items_w: Vec<W> = items_t.into_iter().map(W::new).collect();
          shared_state_clone.update_items(items_w);
          // Reset selection if out of bounds
          let count = shared_state_clone.get_items_count();
          if shared_state_clone.get_selected_index() >= count && count > 0 {
            shared_state_clone.update_selected_index(count - 1);
          } else if count == 0 {
            shared_state_clone.update_selected_index(0);
          }
          if let Some(tx) = tx {
            let _ = tx.send(Action::ItemsLoaded); // Send specific action
          }
        },
        Err(err) => {
          error!("Failed to fetch items: {}", err);
          if let Some(tx) = tx {
            let _ = tx.send(Action::Error(format!("Failed to fetch items: {}", err)));
            let _ = tx.send(Action::LoadingComplete); // Ensure loading stops on error
          }
        },
      }
    });
  }

  fn perform_action_on_selected<F>(&self, action_factory: F)
  where
    F: FnOnce(Arc<dyn GitRepo>, W) -> Option<Box<dyn FnOnce() + Send + 'static>>, // Use Box<dyn FnOnce>
  {
    if let Some(selected) = self.get_selected_item_wrapper() {
      // Clone necessary data before moving into the closure
      let repo_clone = self.repo.clone();
      let action_tx_clone = self.action_tx.clone();

      if let Some(operation) = action_factory(repo_clone.clone(), selected) {
        // Pass cloned repo
        if let Some(tx) = action_tx_clone.clone() {
          // Clone tx for setting loading
          let _ = tx.send(Action::SetLoading(true));
        }
        spawn(async move {
          operation();
          // Send action on completion
          if let Some(tx) = action_tx_clone {
            // Use cloned tx
            let _ = tx.send(Action::Refresh); // Or a more specific completion action
            let _ = tx.send(Action::SetLoading(false));
          }
        });
      }
    }
  }

  fn perform_bulk_action<F>(&self, action_factory: F)
  where
    F: FnOnce(Arc<dyn GitRepo>, Vec<W>) -> Option<Box<dyn FnOnce() + Send + 'static>>, // Use Box<dyn FnOnce>
  {
    let staged_items = self.shared_state.get_staged_for_deletion();
    if !staged_items.is_empty() {
      let repo_clone = self.repo.clone();
      let action_tx_clone = self.action_tx.clone();

      if let Some(operation) = action_factory(repo_clone.clone(), staged_items) {
        // Pass cloned repo
        if let Some(tx) = action_tx_clone.clone() {
          // Clone tx for setting loading
          let _ = tx.send(Action::SetLoading(true)); // Or specific progress state
        }
        spawn(async move {
          operation();
          // Send action on completion
          if let Some(tx) = action_tx_clone {
            // Use cloned tx
            let _ = tx.send(Action::Refresh);
            let _ = tx.send(Action::SetLoading(false));
          }
        });
      }
    }
  }

  // --- Rendering ---

  fn render_list(&mut self, f: &mut Frame<'_>, area: Rect) {
    let items_w = self.shared_state.get_items(); // Get wrapped items

    // Render items using the wrapper's render method
    let render_items: Vec<ListItem> = items_w.iter().map(|item| item.render()).collect();

    let mut title: String = "Items".to_string(); // Generic title
    match self.loading {
      LoadingOperation::Loading(time) => title = format!("Loading... ({})", format_time_elapsed(time)),
      LoadingOperation::Processing(time) => title = format!("Processing... ({})", format_time_elapsed(time)),
      LoadingOperation::ProcessingWithProgress(time, current, total) => {
        title = format!("Processing {}/{}... ({})", current, total, format_time_elapsed(time))
      },
      LoadingOperation::None => {}, // Keep default title
    }

    let list = List::new(render_items)
      .block(Block::default().title(title.to_string()).borders(Borders::ALL)) // Use title.as_str()
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().add_modifier(Modifier::BOLD))
      .highlight_symbol("→")
      .repeat_highlight_symbol(true);

    // Ensure list_state selection is valid
    let count = self.shared_state.get_items_count();
    let current_selection = self.list_state.selected();

    if count == 0 {
      self.list_state.select(None);
    } else {
      let current_idx = current_selection.unwrap_or(0);
      let max_idx = count - 1;
      if current_idx > max_idx {
        self.list_state.select(Some(max_idx));
      } else if current_selection.is_none() {
        // Select based on shared state if nothing selected in list_state
        self.list_state.select(Some(self.shared_state.get_selected_index().min(max_idx)));
      }
      // Otherwise, keep existing valid selection
    }

    f.render_stateful_widget(list, area, &mut self.list_state);
  }
}

// --- Component Implementations ---

impl<W, T, DS, AH, IH> Component for GenericListComponent<W, T, DS, AH, IH>
where
  W: ListItemWrapper<T> + Debug + Clone + Send + Sync + 'static, // Ensure W is fully bounded
  T: ManagedItem + Debug + Clone + Send + Sync + 'static,
  DS: ListDataSource<T> + Default + Send + Sync + 'static,
  AH: ListActionHandler<W, T> + Default + Send + Sync + 'static,
  IH: InputHandler<T> + Default + Send + Sync + 'static,
{
  fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
    self.action_tx = Some(tx);
    // Trigger initial load
    self.send_action(Action::Refresh);
    Ok(())
  }

  fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
    // self.sync_state_for_render(); // No longer needed?

    let constraints = if self.mode == Mode::Input {
      vec![Constraint::Min(1), Constraint::Length(3), Constraint::Length(3)] // List, Input, Footer
    } else {
      vec![Constraint::Min(1), Constraint::Length(3)] // List, Footer
    };

    let chunks = Layout::default().direction(Direction::Vertical).constraints(constraints).split(area);

    self.render_list(frame, chunks[0]);

    if self.mode == Mode::Input {
      self.input_component.render(frame, chunks[1]);
    }

    // TODO: Refactor InstructionFooter
    let footer_chunk = *chunks.last().unwrap();
    let selected_item_wrapper = self.get_selected_item_wrapper();
    let has_staged = self.shared_state.has_staged_items();
    let instructions = self.action_handler.get_instructions(selected_item_wrapper.as_ref(), has_staged);
    let footer_text = instructions.join(" | ");
    let footer_paragraph = ratatui::widgets::Paragraph::new(footer_text)
      .block(Block::default().borders(Borders::ALL))
      .style(Style::default().fg(Color::White));
    frame.render_widget(footer_paragraph, footer_chunk);

    Ok(())
  }
}

#[async_trait]
impl<W, T, DS, AH, IH> AsyncComponent for GenericListComponent<W, T, DS, AH, IH>
where
  W: ListItemWrapper<T> + Debug + Clone + Send + Sync + 'static, // Ensure W is fully bounded
  T: ManagedItem + Debug + Clone + Send + Sync + 'static,
  DS: ListDataSource<T> + Default + Send + Sync + 'static,
  AH: ListActionHandler<W, T> + Default + Send + Sync + 'static,
  IH: InputHandler<T> + Default + Send + Sync + 'static,
{
  async fn handle_events(&mut self, event: Option<crate::tui::Event>) -> Result<Option<Action>> {
    match event {
      Some(crate::tui::Event::Key(key)) => {
        if self.mode == Mode::Input {
          // Let input component handle keys first
          if let Some(action) = self.input_component.handle_input_event(key).await {
            Ok(Some(action))
          } else {
            Ok(None) // Input component consumed the key but didn't yield an action
          }
        } else {
          // Selection mode: handle navigation and delegate others to action handler
          match key.code {
            KeyCode::Up => Ok(Some(Action::SelectPrevious)), // Generic actions
            KeyCode::Down => Ok(Some(Action::SelectNext)),
            // TODO: Add PageUp, PageDown, Home, End if desired
            _ => {
              // Delegate other keys to the specific action handler
              let selected = self.get_selected_item_wrapper();
              self.action_handler.handle_key_event(key, selected.as_ref()).await
            },
          }
        }
      },
      _ => Ok(None), // Ignore non-key events for now
    }
  }

  async fn update(&mut self, action: Action) -> Result<Option<Action>> {
    match action {
      // --- Loading/State ---
      Action::Refresh => {
        self.load_items();
        Ok(None)
      },
      Action::ItemsLoaded => {
        self.set_loading(LoadingOperation::None);
        // Update input component's view of items
        let items_t = self.shared_state.get_items().iter().map(|w| w.inner_item().clone()).collect::<Vec<T>>();
        self.input_component.update_current_items(Arc::new(items_t));
        Ok(Some(Action::Render))
      },
      Action::LoadingComplete => {
        self.set_loading(LoadingOperation::None);
        Ok(Some(Action::Render))
      },
      Action::SetLoading(loading) => {
        // Example action to set loading externally
        self.set_loading(if loading {
          LoadingOperation::Processing(SystemTime::now())
        } else {
          LoadingOperation::None
        });
        Ok(None)
      },

      // --- Mode Changes ---
      Action::InitNewBranch | Action::InitNewStash => {
        // Handle generic init actions
        self.mode = Mode::Input;
        self.input_component.reset(); // Reset input field
        Ok(Some(Action::StartInputMode)) // Use generic action if needed elsewhere
      },
      Action::EndInputMod => {
        self.mode = Mode::Selection;
        Ok(Some(Action::Render))
      },

      // --- Navigation ---
      Action::SelectNext => {
        self.select_next();
        Ok(Some(Action::Render))
      },
      Action::SelectPrevious => {
        self.select_previous();
        Ok(Some(Action::Render))
      },

      // --- Item Actions (Delegated) ---
      // These specific actions should ideally be triggered by the key handler returning them
      Action::CheckoutSelectedBranch | Action::ApplySelectedStash => {
        self.perform_action_on_selected(|repo, item| {
          self
            .action_handler
            .handle_primary_action(repo, item)
            .map(|f| Box::new(f) as Box<dyn FnOnce() + Send + 'static>)
        });
        Ok(None)
      },
      Action::PopSelectedStash => {
        // Pop needs special handling - add handle_pop_action to ListActionHandler trait
        // Or handle it via a specific keybinding returning Action::PopSelectedStash
        info!("Pop action needs specific handling logic");
        // Example:
        // self.perform_action_on_selected(|repo, item| {
        //     self.action_handler.handle_pop_action(repo, item).map(|f| Box::new(f) as Box<dyn FnOnce() + Send + 'static>)
        // });
        Ok(None)
      },
      Action::DeleteBranch | Action::DropSelectedStash => {
        self.perform_action_on_selected(|repo, item| {
          self
            .action_handler
            .handle_delete_action(repo, item)
            .map(|f| Box::new(f) as Box<dyn FnOnce() + Send + 'static>)
        });
        Ok(None)
      },
      Action::DeleteStagedBranches | Action::DeleteStagedStashes => {
        self.perform_bulk_action(|repo, items| {
          self
            .action_handler
            .handle_bulk_delete_action(repo, items)
            .map(|f| Box::new(f) as Box<dyn FnOnce() + Send + 'static>)
        });
        Ok(None)
      },

      // --- Staging ---
      Action::StageBranchForDeletion | Action::StageStashForDeletion => {
        let index = self.shared_state.get_selected_index();
        self.shared_state.stage_item_for_deletion(index, true);
        Ok(Some(Action::Render))
      },
      Action::UnstageBranchForDeletion | Action::UnstageStashForDeletion => {
        let index = self.shared_state.get_selected_index();
        self.shared_state.stage_item_for_deletion(index, false);
        Ok(Some(Action::Render))
      },

      // --- Creation (Triggered by Input Component via Action) ---
      Action::CreateBranch(name) | Action::CreateStash(name) => {
        // The action handler's post_create_action should return the correct action (e.g. CreateBranch)
        // We might need a more robust way to link the input submission to the final action.
        // For now, assume the action handler's get_post_create_action was used correctly.
        info!("Handling creation action for: {}", name);
        // TODO: Implement the actual creation logic, potentially by calling a method on action_handler
        // that returns a closure, similar to other actions.
        // Example:
        // if let Some(operation) = self.action_handler.handle_create_action(self.repo.clone(), name) {
        //     spawn(async move { operation(); /* TODO: Send Refresh */ });
        // }
        self.mode = Mode::Selection; // Switch back to selection mode after triggering creation
        Ok(Some(Action::Refresh)) // Refresh list after creation attempt
      },

      // --- Input Handling (Forwarded from handle_events) ---
      // These might not be needed if input component directly returns CreateBranch/CreateStash
      // Action::UpdateNewBranchName(key) | Action::UpdateNewStashMessage(key) => {
      //     // Forward key event to the input component when in input mode
      //     if self.mode == Mode::Input {
      //         self.input_component.handle_input_event(key).await
      //     } else {
      //         Ok(None) // Should not happen if mode logic is correct
      //     }
      // }

      // --- Default ---
      _ => Ok(None), // Ignore other actions for now
    }
  }
}

// Helper function (consider moving to utils)
fn format_time_elapsed(time: SystemTime) -> String {
  match time.elapsed() {
    Ok(elapsed) => format!("{:.1}s", elapsed.as_secs_f64()),
    Err(err) => {
      warn!("Failed to get system time {}", err);
      String::from("xs")
    },
  }
}
