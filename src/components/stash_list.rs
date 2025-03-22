use crossterm::event::KeyEvent;
use ratatui::{
  Frame,
  layout::Rect,
  style::{Color, Modifier, Style},
  text::{Line, Span},
  widgets::{Block, Borders, List, ListItem, ListState},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  action::Action,
  components::Component,
  git::git_repo::{GitRepo, GitStash},
};

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy)]
enum LoadingOperation {
  None,
  LoadingStashes,
}

pub struct StashList {
  stashes: Vec<StashItem>,
  list_state: ListState,
  loading: LoadingOperation,
  action_tx: Option<UnboundedSender<Action>>,
  repo: Box<dyn GitRepo>,
}

impl StashList {
  pub fn new(repo: Box<dyn GitRepo>) -> Self {
    StashList {
      stashes: Vec::new(),
      list_state: ListState::default(),
      loading: LoadingOperation::None,
      action_tx: None,
      repo,
    }
  }

  pub fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
    self.action_tx = Some(tx);
    Ok(())
  }

  pub async fn load_stashes(&mut self) -> color_eyre::Result<()> {
    self.loading = LoadingOperation::LoadingStashes;
    if let Some(tx) = &self.action_tx {
      tx.send(Action::Render).unwrap();
    }

    let stashes = self.repo.stashes().await?;
    self.stashes = stashes.iter().map(|stash| StashItem::new(stash.clone())).collect();

    self.loading = LoadingOperation::None;
    if let Some(tx) = &self.action_tx {
      tx.send(Action::Render).unwrap();
    }

    Ok(())
  }
}

#[async_trait::async_trait]
impl Component for StashList {
  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
    let render_items: Vec<ListItem> = self.stashes.iter().map(|stash| stash.render()).collect();

    let title = match self.loading {
      LoadingOperation::LoadingStashes => "Loading Stashes...",
      LoadingOperation::None => "Stashes",
    };

    let list = List::new(render_items)
      .block(Block::default().title(title).borders(Borders::ALL))
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().add_modifier(Modifier::BOLD))
      .highlight_symbol("→")
      .repeat_highlight_symbol(true);
    f.render_stateful_widget(list, area, &mut self.list_state);
    Ok(())
  }

  async fn handle_key_events(&mut self, _key: KeyEvent) -> color_eyre::Result<Option<Action>> {
    Ok(None)
  }

  async fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
    match action {
      Action::Refresh => {
        self.load_stashes().await?;
        Ok(None)
      },
      _ => Ok(None),
    }
  }
}
