use crossterm::event::KeyCode;
use ratatui::{
  prelude::*,
  widgets::{Block, Borders, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  action::Action,
  components::{AsyncComponent, Component},
};

#[derive(Default)]
pub struct ErrorView {
  message: Option<String>,
  scroll: u16,
  last_height: u16,
}

impl ErrorView {
  pub fn set_message(&mut self, message: String) {
    self.message = Some(message);
  }

  fn has_scrolled_to_bottom(&self) -> bool {
    match &self.message {
      Some(message) => {
        let total_lines = message.lines().count() as u16;
        self.scroll + self.last_height >= total_lines
      },
      None => false,
    }
  }
}

impl Component for ErrorView {
  fn register_action_handler(&mut self, _tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
    Ok(())
  }

  fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
    self.last_height = area.height.saturating_sub(2);
    let message = self.message.clone().unwrap_or_default();
    let paragraph = Paragraph::new(message)
      .block(Block::default().title("Error").style(Style::default().fg(Color::Red)).borders(Borders::ALL))
      .scroll((self.scroll, 0));

    frame.render_widget(paragraph, area);
    Ok(())
  }
}

#[async_trait::async_trait]
impl AsyncComponent for ErrorView {
  async fn handle_events(&mut self, event: Option<crate::tui::Event>) -> color_eyre::Result<Option<Action>> {
    match event {
      Some(crate::tui::Event::Key(key)) => {
        let action = match key.code {
          KeyCode::Up | KeyCode::Char('w' | 'W') => {
            if self.scroll > 0 {
              self.scroll -= 1;
            }
            None
          },
          KeyCode::Down | KeyCode::Char('s' | 'S') => {
            if !self.has_scrolled_to_bottom() {
              self.scroll += 1;
            }
            None
          },
          _ => {
            self.scroll = 0;
            self.message = None;
            self.last_height = 0;
            Some(Action::ExitError)
          },
        };
        Ok(action)
      },
      _ => Ok(None),
    }
  }

  async fn update(&mut self, _action: Action) -> color_eyre::Result<Option<Action>> {
    Ok(None)
  }
}
