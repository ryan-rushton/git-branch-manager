/// A marker trait for items that can be managed by the generic list component.
/// This could be expanded later to include common methods if needed (e.g., get_id).
pub trait ManagedItem: Clone + Send + Sync + 'static {}
