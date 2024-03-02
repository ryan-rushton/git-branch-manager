use crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Display, Deserialize)]
pub enum Action {
  CreateBranch(String),
  DeleteBranch,
  DeleteStagedBranches,
  Error(String),
  StartInputMode,
  EndInputMod,
  Quit,
  InitNewBranch,
  UpdateNewBranchName(KeyEvent),
  Refresh,
  Render,
  Resize(u16, u16),
  Resume,
  SelectNextBranch,
  SelectPreviousBranch,
  StageBranchForDeletion,
  Suspend,
  Tick,
  UnstageBranchForDeletion,
}
