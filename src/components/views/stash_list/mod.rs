// mod instruction_footer; // Removed, using shared footer
// pub mod list; // Removed, replaced by type alias below
mod stash_action_handler;
mod stash_data_source;
// mod stash_input; // Removed, using GenericInputComponent
mod stash_input_handler;
mod stash_item;

// pub use instruction_footer::InstructionFooter; // Removed
// pub use list::StashList; // Removed
pub use stash_action_handler::StashActionHandler;
pub use stash_data_source::StashDataSource;
// pub use stash_input::StashInput; // Removed
pub use stash_input_handler::StashInputHandler;
pub use stash_item::StashItem;

use crate::{components::shared::generic_list::GenericListComponent, git::types::GitStash};

// Type alias for the specific Stash List implementation using the generic component
pub type StashListComponent = GenericListComponent<
  StashItem,          // Wrapper type
  GitStash,           // Item type
  StashDataSource,    // Data source implementation
  StashActionHandler, // Action handler implementation
  StashInputHandler,  // Input handler implementation
>;
