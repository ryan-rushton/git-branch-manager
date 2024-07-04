use ratatui::{
  layout::Rect,
  style::{Color, Modifier, Style},
  text::{Line, Span},
  widgets::{Block, Borders, List, ListItem, ListState},
  Frame,
};

use crate::{
  components::Component,
  git::git_repo::{GitRepo, GitStash},
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct StashItem {
  git_stash: GitStash,
}

impl StashItem {
  pub fn new(git_stash: GitStash) -> Self {
    StashItem { git_stash }
  }

  pub fn render(&self) -> ListItem {
    let mut text = Line::default();
    let mut parts = Vec::new();
    let index = Span::styled(self.git_stash.index.to_string(), Style::default());
    parts.push(index);

    let message =
      Span::styled(format!(" {}", self.git_stash.message.clone()), Style::default().add_modifier(Modifier::DIM));
    parts.push(message);

    let id =
      Span::styled(format!(" ({})", self.git_stash.stash_id.clone()), Style::default().add_modifier(Modifier::DIM));
    parts.push(id);

    text = text.spans(parts);
    ListItem::from(text)
  }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StashList {
  stashes: Vec<StashItem>,
  list_state: ListState,
}

impl StashList {
  pub fn new(mut repo: Box<dyn GitRepo>) -> Self {
    let stashes: Vec<StashItem> =
      repo.stashes().unwrap().iter().map(|git_stash| StashItem::new(git_stash.clone())).collect();
    StashList { stashes, list_state: ListState::default() }
  }
}

impl Component for StashList {
  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
    let render_items: Vec<ListItem> = self.stashes.iter().map(|stash| stash.render()).collect();
    let list = List::new(render_items)
      .block(Block::default().title("Stashes").borders(Borders::ALL))
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().add_modifier(Modifier::BOLD))
      .highlight_symbol("→")
      .repeat_highlight_symbol(true);
    f.render_stateful_widget(list, area, &mut self.list_state);
    Ok(())
  }
}
