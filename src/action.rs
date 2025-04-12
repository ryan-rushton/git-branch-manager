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
  ItemsLoaded,     // Added for generic list
  LoadingComplete, // Added for generic list
  ExitError,
  InitNewBranch,
  InitNewStash,
  Quit,
  Refresh,
  Render,
  Resize(u16, u16),
  Resume,
  SelectNext,     // Generic selection
  SelectPrevious, // Generic selection
  StageBranchForDeletion,
  StartInputMode,
  SetLoading(bool), // Added for generic list
  Suspend,
  Tick,
  ToggleView,
  UnstageBranchForDeletion,
  // UpdateNewBranchName(KeyEvent), // Removed, handled internally by GenericInputComponent
  // SelectNextStash, // Removed, use SelectNext
  // SelectPreviousStash, // Removed, use SelectPrevious
  ApplySelectedStash,
  PopSelectedStash,
  DropSelectedStash,
  StageStashForDeletion,
  UnstageStashForDeletion,
  DeleteStagedStashes,
  CreateStash(String),
}
