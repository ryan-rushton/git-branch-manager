// pub mod list; // Removed, replaced by type alias below

mod branch_action_handler;
mod branch_data_source;
// mod branch_input; // Removed, using GenericInputComponent
mod branch_input_handler;
mod branch_item;
// mod instruction_footer; // Removed, using shared footer

pub use branch_action_handler::BranchActionHandler;
pub use branch_data_source::BranchDataSource;
// pub use branch_input::BranchInput; // Removed
pub use branch_input_handler::BranchInputHandler;
pub use branch_item::BranchItem;

// pub use instruction_footer::InstructionFooter; // Removed
use crate::{components::shared::generic_list::GenericListComponent, git::types::GitBranch};

// Type alias for the specific Branch List implementation using the generic component
pub type BranchListComponent = GenericListComponent<
  BranchItem,          // Wrapper type
  GitBranch,           // Item type
  BranchDataSource,    // Data source implementation
  BranchActionHandler, // Action handler implementation
  BranchInputHandler,  // Input handler implementation
>;
