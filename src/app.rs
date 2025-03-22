use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::Rect;
use tokio::sync::mpsc;

use crate::{
  action::Action,
  components::{Component, branch_list::BranchList, stash_list::StashList},
  config::Config,
  git::{git_cli_repo::GitCliRepo, git2_repo::Git2Repo},
  mode::Mode,
  tui,
};

pub enum View {
  Branches,
  Stashes,
}

pub struct App {
  pub config: Config,
  pub branch_list: Box<dyn Component>,
  pub stash_list: Box<dyn Component>,
  pub should_quit: bool,
  pub should_suspend: bool,
  pub mode: Mode,
  pub view: View,
}

impl App {
  pub fn new() -> Result<Self> {
    let config = Config::new()?;
    let branch_list = Box::new(BranchList::new(Box::new(GitCliRepo::from_cwd().unwrap())));
    let stash_list = Box::new(StashList::new(Box::new(Git2Repo::from_cwd().unwrap())));
    let mode = Mode::Default;
    Ok(Self { config, branch_list, stash_list, should_quit: false, should_suspend: false, mode, view: View::Branches })
  }

  pub async fn run(&mut self) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();

    // Initialize the terminal user interface
    let mut tui = tui::Tui::new()?;
    tui.enter()?;

    // Register action handlers for components
    self.branch_list.register_action_handler(action_tx.clone())?;
    self.stash_list.register_action_handler(action_tx.clone())?;

    // Initial load of data
    action_tx.send(Action::Refresh)?;

    // Start the main loop
    while !self.should_quit {
      // Render the user interface
      tui.draw(|f| {
        let chunks = ratatui::layout::Layout::default()
          .direction(ratatui::layout::Direction::Vertical)
          .constraints([ratatui::layout::Constraint::Percentage(100)].as_ref())
          .split(f.area());

        match self.view {
          View::Branches => self.branch_list.draw(f, chunks[0]).unwrap(),
          View::Stashes => self.stash_list.draw(f, chunks[0]).unwrap(),
        }
      })?;

      // Handle events
      if let Some(event) = tui.next().await {
        match event {
          tui::Event::Quit => action_tx.send(Action::Quit)?,
          tui::Event::Error => action_tx.send(Action::Error("Unknown error".to_string()))?,
          tui::Event::Tick => action_tx.send(Action::Tick)?,
          tui::Event::Render => action_tx.send(Action::Render)?,
          tui::Event::Key(key) => {
            if key.kind == crossterm::event::KeyEventKind::Press {
              self.handle_key_events(key).await?;
            }
          },
          tui::Event::Mouse(_) => {},
          tui::Event::Resize(w, h) => action_tx.send(Action::Resize(w, h))?,
          _ => {},
        }
      }

      // Handle actions
      while let Ok(action) = action_rx.try_recv() {
        match action {
          Action::Quit => self.should_quit = true,
          Action::Suspend => self.should_suspend = true,
          Action::Resume => self.should_suspend = false,
          Action::Refresh => {
            match self.view {
              View::Branches => {
                if let Some(next_action) = self.branch_list.update(action).await? {
                  action_tx.send(next_action)?;
                }
              },
              View::Stashes => {
                if let Some(next_action) = self.stash_list.update(action).await? {
                  action_tx.send(next_action)?;
                }
              },
            }
          },
          Action::Render => {
            tui.draw(|f| {
              let chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([ratatui::layout::Constraint::Percentage(100)].as_ref())
                .split(f.area());

              match self.view {
                View::Branches => self.branch_list.draw(f, chunks[0]).unwrap(),
                View::Stashes => self.stash_list.draw(f, chunks[0]).unwrap(),
              }
            })?;
          },
          Action::Resize(w, h) => {
            tui.resize(Rect::new(0, 0, w, h))?;
          },
          Action::Error(e) => {
            // TODO: Handle error
            println!("Error: {}", e);
          },
          Action::Tick | Action::StartInputMode | Action::EndInputMod => {},
          _ => {
            match self.view {
              View::Branches => {
                if let Some(next_action) = self.branch_list.update(action).await? {
                  action_tx.send(next_action)?;
                }
              },
              View::Stashes => {
                if let Some(next_action) = self.stash_list.update(action).await? {
                  action_tx.send(next_action)?;
                }
              },
            }
          },
        }
      }

      if self.should_suspend {
        tui.suspend()?;
        action_tx.send(Action::Resume)?;
        tui = tui::Tui::new()?;
        tui.enter()?;
      }
    }

    // Exit the terminal interface
    tui.exit()?;
    Ok(())
  }

  async fn handle_key_events(&mut self, key: KeyEvent) -> Result<()> {
    match key {
      KeyEvent { code: KeyCode::Char('q'), modifiers: KeyModifiers::NONE, .. } => {
        self.should_quit = true;
      },
      KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. } => {
        self.should_quit = true;
      },
      KeyEvent { code: KeyCode::Char('z'), modifiers: KeyModifiers::CONTROL, .. } => {
        self.should_suspend = true;
      },
      KeyEvent { code: KeyCode::Tab, modifiers: KeyModifiers::NONE, .. } => {
        self.view = match self.view {
          View::Branches => View::Stashes,
          View::Stashes => View::Branches,
        };
      },
      _ => {
        match self.view {
          View::Branches => {
            if let Some(action) = self.branch_list.handle_key_events(key).await? {
              if let Some(next_action) = self.branch_list.update(action).await? {
                match next_action {
                  Action::StartInputMode => self.mode = Mode::Input,
                  Action::EndInputMod => self.mode = Mode::Default,
                  _ => {},
                }
              }
            }
          },
          View::Stashes => {
            if let Some(action) = self.stash_list.handle_key_events(key).await? {
              if let Some(next_action) = self.stash_list.update(action).await? {
                match next_action {
                  Action::StartInputMode => self.mode = Mode::Input,
                  Action::EndInputMod => self.mode = Mode::Default,
                  _ => {},
                }
              }
            }
          },
        }
      },
    }
    Ok(())
  }
}
