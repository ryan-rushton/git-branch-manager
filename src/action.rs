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
  InitNewStash,
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
  ToggleView,
  UnstageBranchForDeletion,
  UpdateNewBranchName(KeyEvent),
  SelectNextStash,
  SelectPreviousStash,
  ApplySelectedStash,
  PopSelectedStash,
  DropSelectedStash,
  StageStashForDeletion,
  UnstageStashForDeletion,
  DeleteStagedStashes,
  CreateStash(String),
}
