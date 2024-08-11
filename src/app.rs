use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::Rect;
use tokio::sync::mpsc;

use crate::{
  action::Action,
  components::{branch_list::BranchList, stash_list::StashList, Component},
  git::git2_repo::Git2Repo,
  mode::Mode,
  tui,
  tui::Tui,
};

pub enum View {
  Branches,
  Stashes,
}

const TICK_RATE: f64 = 10.0;
const FRAME_RATE: f64 = 30.0;

pub struct App {
  pub branch_list: Box<dyn Component>,
  pub stash_list: Box<dyn Component>,
  pub should_quit: bool,
  pub should_suspend: bool,
  pub mode: Mode,
  pub view: View,
}

impl App {
  pub fn new() -> Result<Self> {
    // TODO only have a single repo that is shared
    let branch_list = Box::new(BranchList::new(Box::new(Git2Repo::from_cwd().unwrap())));
    let stash_list = Box::new(StashList::new(Box::new(Git2Repo::from_cwd().unwrap())));
    let mode = Mode::Default;
    Ok(Self { branch_list, stash_list, should_quit: false, should_suspend: false, mode, view: View::Branches })
  }

  pub async fn run(&mut self) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();

    let mut tui = tui::Tui::new()?.tick_rate(TICK_RATE).frame_rate(FRAME_RATE);
    // tui.mouse(true);
    tui.enter()?;

    self.branch_list.register_action_handler(action_tx.clone())?;
    self.stash_list.register_action_handler(action_tx.clone())?;

    loop {
      if let Some(e) = tui.next().await {
        match e {
          tui::Event::Quit => action_tx.send(Action::Quit)?,
          tui::Event::Tick => action_tx.send(Action::Tick)?,
          tui::Event::Render => action_tx.send(Action::Render)?,
          tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
          tui::Event::Key(key) => {
            if self.mode == Mode::Default {
              let action = match key {
                KeyEvent { code: KeyCode::Char('q'), modifiers: _, state: _, kind: _ } => Some(Action::Quit),
                KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::CONTROL, state: _, kind: _ } => {
                  Some(Action::Quit)
                },
                _ => None,
              };
              if action.is_some() {
                action_tx.send(action.unwrap())?;
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
          Action::Quit => self.should_quit = true,
          Action::Suspend => self.should_suspend = true,
          Action::Resume => self.should_suspend = false,
          Action::Resize(w, h) => {
            tui.resize(Rect::new(0, 0, w, h))?;
            tui.draw(|f| {
              let r = component.draw(f, f.area());
              if let Err(e) = r {
                action_tx.send(Action::Error(format!("Failed to draw: {:?}", e))).unwrap();
              }
            })?;
          },
          Action::Render => {
            tui.draw(|f| {
              let r = component.draw(f, f.area());
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
