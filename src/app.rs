use std::sync::Arc;

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::Rect;
use tokio::sync::mpsc;

use crate::{
  action::Action,
  components::{Component, branch_list::BranchList, error_component::ErrorComponent, stash_list::StashList},
  config::Config,
  git::git_cli_repo::GitCliRepo,
  mode::Mode,
  tui::{self, Tui},
};

pub enum View {
  Branches,
  Stashes,
}

const TICK_RATE: f64 = 10.0;
const FRAME_RATE: f64 = 30.0;

pub struct App {
  pub config: Config,
  pub branch_list: Box<dyn Component>,
  pub stash_list: Box<dyn Component>,
  pub error_component: ErrorComponent,
  pub should_quit: bool,
  pub should_suspend: bool,
  pub mode: Mode,
  pub view: View,
}

impl App {
  pub fn new() -> Result<Self> {
    let config = Config::new()?;
    let git_repo = GitCliRepo::from_cwd().map_err(|e| color_eyre::eyre::eyre!(e.to_string()))?;
    let branch_list = Box::new(BranchList::new(Arc::new(git_repo.clone())));
    let stash_list = Box::new(StashList::new(Box::new(git_repo)));
    let error_component = ErrorComponent::default();
    let mode = Mode::Default;
    Ok(Self {
      config,
      branch_list,
      stash_list,
      error_component,
      should_quit: false,
      should_suspend: false,
      mode,
      view: View::Branches,
    })
  }

  pub async fn run(&mut self) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();

    let mut tui = tui::Tui::new()?.tick_rate(TICK_RATE).frame_rate(FRAME_RATE);
    tui.enter()?;

    self.branch_list.register_action_handler(action_tx.clone())?;
    self.stash_list.register_action_handler(action_tx.clone())?;

    // Initial refresh to load data
    action_tx.send(Action::Refresh)?;

    loop {
      if let Some(e) = tui.next().await {
        match e {
          tui::Event::Quit => action_tx.send(Action::Quit)?,
          tui::Event::Tick => action_tx.send(Action::Tick)?,
          tui::Event::Render => action_tx.send(Action::Render)?,
          tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
          tui::Event::Key(key) => {
            match key {
              KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::CONTROL, state: _, kind: _ } => {
                action_tx.send(Action::Quit)?
              },
              _ => {
                match self.mode {
                  Mode::Error => {},
                  Mode::Input => {},
                  Mode::Default => {
                    if let KeyEvent { code: KeyCode::Esc, modifiers: _, state: _, kind: _ } = key {
                      action_tx.send(Action::Quit)?
                    }
                  },
                }
              },
            }
          },
          _ => {},
        }

        let maybe_action = match self.mode {
          Mode::Error => self.error_component.handle_events(Some(e.clone())).await?,
          _ => {
            let component: &mut Box<dyn Component> = match self.view {
              View::Branches => &mut self.branch_list,
              View::Stashes => &mut self.stash_list,
            };
            component.handle_events(Some(e.clone())).await?
          },
        };
        if let Some(action) = maybe_action {
          action_tx.send(action)?;
        }
      }

      while let Ok(action) = action_rx.try_recv() {
        if action != Action::Tick && action != Action::Render {
          log::debug!("{action:?}");
        }
        let component: &mut Box<dyn Component> = match self.view {
          View::Branches => &mut self.branch_list,
          View::Stashes => &mut self.stash_list,
        };

        match &action {
          Action::StartInputMode => self.mode = Mode::Input,
          Action::EndInputMod => self.mode = Mode::Default,
          Action::Quit => self.should_quit = true,
          Action::Suspend => self.should_suspend = true,
          Action::Resume => self.should_suspend = false,
          Action::Error(message) => {
            self.mode = Mode::Error;
            self.error_component.set_message(message.clone());
            tui.clear()?
          },
          Action::ExitError => {
            self.mode = Mode::Default;
            tui.clear()?
          },
          Action::Resize(w, h) => {
            tui.resize(Rect::new(0, 0, *w, *h))?;
            tui.clear()?;
            action_tx.send(Action::Render)?
          },
          Action::Render => {
            tui.draw(|f| {
              let result = match self.mode {
                Mode::Error => self.error_component.draw(f, f.area()),
                _ => component.draw(f, f.area()),
              };
              if let Err(e) = result {
                action_tx.send(Action::Error(format!("Failed to draw: {:?}", e))).unwrap();
              }
            })?;
          },
          Action::Refresh => {
            if let Some(next_action) = component.update(action.clone()).await? {
              action_tx.send(next_action)?;
            }
            tui.clear()?;
            action_tx.send(Action::Render)?;
          },
          _ => {},
        }
        if let Some(action) = component.update(action.clone()).await? {
          action_tx.send(action)?
        };
      }
      if self.should_suspend {
        tui.suspend()?;
        action_tx.send(Action::Resume)?;
        tui = Tui::new()?.tick_rate(TICK_RATE).frame_rate(FRAME_RATE);
        tui.enter()?;
      } else if self.should_quit {
        tui.stop()?;
        break;
      }
    }
    tui.exit()?;
    Ok(())
  }
}
