//! JavaScript value handle type

/// Handle to a JavaScript value stored in the persistent runtime
///
/// This is a lightweight handle that references a JavaScript value
/// kept alive in the TypeScript runtime's storage.
///
/// Note: This type is duplicated in merlin-core to avoid circular dependencies.
/// Changes here should be reflected there.
#[derive(Debug, Clone)]
pub struct JsValueHandle {
    /// Unique identifier for this value in the runtime's storage
    pub(crate) id: String,
}

impl JsValueHandle {
    /// Create a new handle with the given ID
    #[must_use]
    pub fn new(id: String) -> Self {
        Self { id }
    }

    /// Get the identifier for this handle
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}
