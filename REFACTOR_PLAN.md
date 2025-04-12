# Refactoring Plan: BranchList and StashList Components

## Analysis of Similarities

- **Core Structure:** Both views (`branch_list` and `stash_list`) use a similar module structure (`list.rs`, `*_item.rs`, `*_input.rs`, `instruction_footer.rs`).
- **`list.rs` Commonalities:**
  - Similar state management (`Mode`, `LoadingOperation`, `SharedState`).
  - Consistent async operation patterns (closures spawning tasks, updating state, sending actions).
  - Similar UI logic (navigation, state sync, rendering, draw structure).
  - Similar action handling (`update` method).
  - Common deletion logic (staging, bulk delete).
  - Shared dependencies (`Arc<dyn GitRepo>`, `UnboundedSender<Action>`).
- **`*_item.rs` Commonalities:** Wrap Git types, manage `staged_for_deletion`, provide `render() -> ListItem`.
- **`*_input.rs` Commonalities:** Use `tui-textarea`, manage `InputState`, handle basic keys, render text area.
- **`instruction_footer.rs` Commonalities:** Render dynamic instructions.

## Analysis of Differences

- **Data Types:** `GitBranch` vs `GitStash`.
- **Specific Git Actions:** Checkout/Create Branch vs Apply/Pop/Drop/Create Stash.
- **Input Validation:** Branch names require validation; stash messages do not.
- **Item Rendering Details:** Different info displayed (HEAD/upstream vs index/message).
- **Keybindings:** Different keys trigger specific actions.
- **Loading States:** Different `LoadingOperation` variants.

## Proposed Refactoring Plan

The core idea is to extract common logic into generic components using traits for specific differences.

1.  **Define Core Traits:**

    - `ManagedItem`: Marker/basic trait for `GitBranch`/`GitStash`.
    - `ListItemWrapper<T: ManagedItem>`: For `BranchItem`/`StashItem` (requires `new`, `render`, `stage_for_deletion`, etc.).
    - `ListDataSource<T: ManagedItem>`: Defines item fetching (`async fn fetch_items(...)`).
    - `ListActionHandler<T: ManagedItem>`: Defines specific actions (`async fn handle_primary_action(...)`, `get_keybindings(...)`, etc.).
    - `InputHandler`: Defines input validation and action creation (`async fn validate(...)`, `fn create_action(...)`).

2.  **Create Generic Components:**

    - `GenericListComponent<ItemWrapper, DataSource, ActionHandler>`: Manages generic `SharedState`, common logic (loading, navigation, rendering, staging), delegates specific actions via `ActionHandler`.
    - `GenericInputComponent<Handler: InputHandler>`: Manages `tui-textarea`, `InputState`, uses `InputHandler` for validation/submit.

3.  **Implement Traits for Branch/Stash:**

    - Implement `ManagedItem` for `GitBranch`/`GitStash`.
    - Implement `ListItemWrapper<GitBranch>` for `BranchItem`, `ListItemWrapper<GitStash>` for `StashItem`.
    - Create `BranchDataSource`, `StashDataSource` implementing `ListDataSource`.
    - Create `BranchActionHandler`, `StashActionHandler` implementing `ListActionHandler`.
    - Create `BranchInputHandler`, `StashInputHandler` implementing `InputHandler`.

4.  **Refactor Instruction Footer:**

    - Create a single `InstructionFooter`.
    - Get keybinding info from the active `ListActionHandler`.

5.  **Update Application Layer:**
    - Instantiate generic components with specific trait implementations based on the active view.

## Visualization

```mermaid
graph TD
    subgraph Shared Components ("src/components/shared")
        direction LR
        GenericListComponent --> SharedState
        GenericListComponent --> GenericInputComponent
        GenericListComponent --> InstructionFooter
        GenericListComponent -- Needs --> ListItemWrapperTrait
        GenericListComponent -- Needs --> ListDataSourceTrait
        GenericListComponent -- Needs --> ListActionHandlerTrait
        GenericInputComponent -- Needs --> InputHandlerTrait
        SharedState -- Manages --> ListItemWrapperTrait
    end

    subgraph Traits ("src/components/traits")
        direction LR
        ManagedItemTrait
        ListItemWrapperTrait
        ListDataSourceTrait
        ListActionHandlerTrait
        InputHandlerTrait
    end

    subgraph Branch Implementation ("src/components/views/branch_list")
        direction LR
        BranchListItself -- Uses --> GenericListComponent_Branch_
        BranchItem -- Implements --> ListItemWrapperTrait
        BranchDataSource -- Implements --> ListDataSourceTrait
        BranchActionHandler -- Implements --> ListActionHandlerTrait
        BranchInputHandler -- Implements --> InputHandlerTrait
        GitBranch -- Implements --> ManagedItemTrait
    end

    subgraph Stash Implementation ("src/components/views/stash_list")
        direction LR
        StashListItself -- Uses --> GenericListComponent_Stash_
        StashItem -- Implements --> ListItemWrapperTrait
        StashDataSource -- Implements --> ListDataSourceTrait
        StashActionHandler -- Implements --> ListActionHandlerTrait
        StashInputHandler -- Implements --> InputHandlerTrait
        GitStash -- Implements --> ManagedItemTrait
    end

    App --> BranchListItself
    App --> StashListItself

    GenericListComponent_Branch_ -- Is a --> GenericListComponent
    GenericListComponent_Stash_ -- Is a --> GenericListComponent

    %% Styling
    classDef shared fill:#ccf,stroke:#333,stroke-width:2px
    classDef trait fill:#ff9,stroke:#333,stroke-width:1px,stroke-dasharray: 5 5
    classDef branch fill:#cfc,stroke:#333,stroke-width:1px
    classDef stash fill:#fcc,stroke:#333,stroke-width:1px

    class GenericListComponent,GenericInputComponent,InstructionFooter,SharedState shared;
    class ManagedItemTrait,ListItemWrapperTrait,ListDataSourceTrait,ListActionHandlerTrait,InputHandlerTrait trait;
    class BranchListItself,BranchItem,BranchDataSource,BranchActionHandler,BranchInputHandler,GitBranch branch;
    class StashListItself,StashItem,StashDataSource,StashActionHandler,StashInputHandler,GitStash stash;
```

## Benefits

- **Reduced Duplication:** Less code repetition.
- **Improved Maintainability:** Changes to common logic made in one place.
- **Easier Extension:** Adding new list views is simpler.
- **Clearer Separation of Concerns:** Generic components handle "how", traits handle "what".
