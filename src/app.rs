use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use tokio::sync::{mpsc, mpsc::UnboundedSender};

use crate::{
  action::Action,
  components::{branch_list::BranchList, stash_list::StashList, Component},
  config::Config,
  git::git2_repo::Git2Repo,
  mode::Mode,
  tui,
  tui::Tui,
};

pub enum View {
  Branches,
  Stashes,
}

pub struct App {
  pub config: Config,
  pub tick_rate: f64,
  pub frame_rate: f64,
  pub branch_list: Box<dyn Component>,
  pub stash_list: Box<dyn Component>,
  pub should_quit: bool,
  pub should_suspend: bool,
  pub mode: Mode,
  pub view: View,
  pub last_tick_key_events: Vec<KeyEvent>,
}

impl App {
  pub fn new(tick_rate: f64, frame_rate: f64) -> Result<Self> {
    // TODO only have a single repo that is shared
    let branch_list = Box::new(BranchList::new(Box::new(Git2Repo::from_cwd().unwrap())));
    let stash_list = Box::new(StashList::new(Box::new(Git2Repo::from_cwd().unwrap())));
    let config = Config::new()?;
    let mode = Mode::Default;
    Ok(Self {
      tick_rate,
      frame_rate,
      branch_list,
      stash_list,
      should_quit: false,
      should_suspend: false,
      config,
      mode,
      view: View::Stashes,
      last_tick_key_events: Vec::new(),
    })
  }

  pub async fn run(&mut self) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();

    let mut tui = tui::Tui::new()?.tick_rate(self.tick_rate).frame_rate(self.frame_rate);
    // tui.mouse(true);
    tui.enter()?;

    setup_component(&mut self.branch_list, &action_tx, &self.config, &tui)?;
    setup_component(&mut self.stash_list, &action_tx, &self.config, &tui)?;

    loop {
      if let Some(e) = tui.next().await {
        match e {
          tui::Event::Quit => action_tx.send(Action::Quit)?,
          tui::Event::Tick => action_tx.send(Action::Tick)?,
          tui::Event::Render => action_tx.send(Action::Render)?,
          tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
          tui::Event::Key(key) => {
            if let Some(keymap) = self.config.keybindings.get(&self.mode) {
              if let Some(action) = keymap.get(&vec![key]) {
                log::info!("Got action: {action:?}");
                action_tx.send(action.clone())?;
              } else {
                // If the key was not handled as a single key action,
                // then consider it for multi-key combinations.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                  log::info!("Got action: {action:?}");
                  action_tx.send(action.clone())?;
                }
              }
            };
          },
          _ => {},
        }

        let component: &mut Box<dyn Component> = match self.view {
          View::Branches => &mut self.branch_list,
          View::Stashes => &mut self.stash_list,
        };
        if let Some(action) = component.handle_events(Some(e.clone()))? {
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

        match action {
          Action::StartInputMode => self.mode = Mode::Input,
          Action::EndInputMod => self.mode = Mode::Default,
          Action::Tick => {
            self.last_tick_key_events.drain(..);
          },
          Action::Quit => self.should_quit = true,
          Action::Suspend => self.should_suspend = true,
          Action::Resume => self.should_suspend = false,
          Action::Resize(w, h) => {
            tui.resize(Rect::new(0, 0, w, h))?;
            tui.draw(|f| {
              let r = component.draw(f, f.size());
              if let Err(e) = r {
                action_tx.send(Action::Error(format!("Failed to draw: {:?}", e))).unwrap();
              }
            })?;
          },
          Action::Render => {
            tui.draw(|f| {
              let r = component.draw(f, f.size());
              if let Err(e) = r {
                action_tx.send(Action::Error(format!("Failed to draw: {:?}", e))).unwrap();
              }
            })?;
          },
          _ => {},
        }
        if let Some(action) = component.update(action.clone())? {
          action_tx.send(action)?
        };
      }
      if self.should_suspend {
        tui.suspend()?;
        action_tx.send(Action::Resume)?;
        tui = tui::Tui::new()?.tick_rate(self.tick_rate).frame_rate(self.frame_rate);
        // tui.mouse(true);
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

fn setup_component(
  component: &mut Box<dyn Component>,
  action_tx: &UnboundedSender<Action>,
  config: &Config,
  tui: &Tui,
) -> Result<()> {
  component.register_action_handler(action_tx.clone())?;
  component.register_config_handler(config.clone())?;
  component.init(tui.size()?)?;
  Ok(())
}
