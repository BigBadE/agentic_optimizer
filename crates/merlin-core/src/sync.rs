//! Synchronization utilities for handling poisoned locks.

use std::sync::{Mutex, MutexGuard};

/// Extension trait for `Mutex` that ignores lock poisoning.
///
/// Lock poisoning occurs when a thread panics while holding a lock. In most cases,
/// the original panic is the real error we care about, not the poisoned lock state.
/// This trait provides methods to acquire locks while ignoring poison errors.
pub trait IgnoreLock<T> {
    /// Lock the mutex, ignoring any poison error.
    ///
    /// If the lock is poisoned, this method will clear the poison and return
    /// the guard anyway. Use this when lock poisoning is not a concern for
    /// your use case.
    fn lock_ignore_poison(&self) -> MutexGuard<'_, T>;
}

impl<T> IgnoreLock<T> for Mutex<T> {
    fn lock_ignore_poison(&self) -> MutexGuard<'_, T> {
        match self.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}
