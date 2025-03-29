use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
  prelude::*,
  widgets::{Block, Borders, Paragraph},
};

use super::Component;
use crate::action::Action;

#[derive(Default)]
pub struct ErrorComponent {
  message: Option<String>,
  scroll: u16,
  last_height: u16,
}

impl ErrorComponent {
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

#[async_trait::async_trait]
impl Component for ErrorComponent {
  fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
    self.last_height = area.height.saturating_sub(2);
    let message = self.message.clone().unwrap_or_default();
    let paragraph = Paragraph::new(message)
      .block(Block::default().title("Error").style(Style::default().fg(Color::Red)).borders(Borders::ALL))
      .scroll((self.scroll, 0));

    frame.render_widget(paragraph, area);
    Ok(())
  }

  async fn handle_key_events(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
    let action = match key.code {
      KeyCode::Up => {
        if self.scroll > 0 {
          self.scroll -= 1;
        }
        None
      },
      KeyCode::Down => {
        if !self.has_scrolled_to_bottom() {
          self.scroll += 1;
        }
        None
      },
      KeyCode::Esc => {
        self.scroll = 0;
        self.message = None;
        self.last_height = 0;
        Some(Action::ExitError)
      },
      _ => None,
    };
    Ok(action)
  }
}
