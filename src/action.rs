use crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Display, Deserialize)]
pub enum Action {
  CheckoutSelectedBranch,
  CreateBranch(String),
  DeleteBranch,
  DeleteStagedBranches,
  EndInputMod,
  Error(String),
  ExitError,
  InitNewBranch,
  Quit,
  Refresh,
  Render,
  Resize(u16, u16),
  Resume,
  SelectNextBranch,
  SelectPreviousBranch,
  StageBranchForDeletion,
  StartInputMode,
  Suspend,
  Tick,
  UnstageBranchForDeletion,
  UpdateNewBranchName(KeyEvent),
}
