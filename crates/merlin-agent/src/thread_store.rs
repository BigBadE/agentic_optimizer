//! Thread persistence and management.
//!
//! Handles saving/loading threads to/from disk and managing thread operations.

use merlin_core::{MessageId, Result, RoutingError, Thread, ThreadColor, ThreadId};
use merlin_deps::serde_json::{from_str, to_string_pretty};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Storage for conversation threads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadStore {
    /// All threads indexed by ID
    threads: HashMap<ThreadId, Thread>,
    /// Path to the storage directory
    #[serde(skip)]
    storage_path: PathBuf,
    /// Next color index for new threads
    next_color_index: usize,
}

impl ThreadStore {
    /// Creates a new thread store with the given storage path
    ///
    /// # Errors
    /// Returns an error if the storage directory cannot be created
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        // Only create directory if it doesn't exist to avoid slow Windows FS operations
        if !storage_path.exists() {
            fs::create_dir_all(&storage_path).map_err(|err| {
                RoutingError::Other(format!("Failed to create thread storage directory: {err}"))
            })?;
        }

        Ok(Self {
            threads: HashMap::new(),
            storage_path,
            next_color_index: 0,
        })
    }

    /// Loads all threads from disk
    ///
    /// # Errors
    /// Returns an error if thread files cannot be read or parsed
    pub fn load_all(&mut self) -> Result<()> {
        let entries = fs::read_dir(&self.storage_path).map_err(|err| {
            RoutingError::Other(format!("Failed to read thread storage directory: {err}"))
        })?;

        for entry in entries {
            let entry = entry.map_err(|err| {
                RoutingError::Other(format!("Failed to read directory entry: {err}"))
            })?;

            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                let contents = fs::read_to_string(&path).map_err(|err| {
                    RoutingError::Other(format!("Failed to read thread file: {err}"))
                })?;

                let thread: Thread = from_str(&contents).map_err(|err| {
                    RoutingError::Other(format!("Failed to parse thread JSON: {err}"))
                })?;

                self.threads.insert(thread.id, thread);
            }
        }

        Ok(())
    }

    /// Saves a thread to disk
    ///
    /// # Errors
    /// Returns an error if the thread cannot be serialized or written to disk
    pub fn save_thread(&mut self, thread: &Thread) -> Result<()> {
        self.threads.insert(thread.id, thread.clone());

        let path = self.thread_path(thread.id);
        let json = to_string_pretty(thread)
            .map_err(|err| RoutingError::Other(format!("Failed to serialize thread: {err}")))?;

        fs::write(&path, json)
            .map_err(|err| RoutingError::Other(format!("Failed to write thread file: {err}")))?;

        Ok(())
    }

    /// Creates a new thread with automatic color assignment
    pub fn create_thread(&mut self, name: String) -> Thread {
        let color = ThreadColor::from_index(self.next_color_index);
        self.next_color_index += 1;
        Thread::new(name, color)
    }

    /// Creates a new thread branched from another thread
    ///
    /// # Errors
    /// Returns an error if the parent thread doesn't exist
    pub fn create_branch(
        &mut self,
        name: String,
        parent_thread_id: ThreadId,
        parent_message_id: MessageId,
    ) -> Result<Thread> {
        // Verify parent thread exists
        if !self.threads.contains_key(&parent_thread_id) {
            return Err(RoutingError::Other(format!(
                "Parent thread {parent_thread_id} not found"
            )));
        }

        let color = ThreadColor::from_index(self.next_color_index);
        self.next_color_index += 1;

        Ok(Thread::branched_from(
            name,
            color,
            parent_thread_id,
            parent_message_id,
        ))
    }

    /// Gets a thread by ID
    #[must_use]
    pub fn get_thread(&self, thread_id: ThreadId) -> Option<&Thread> {
        self.threads.get(&thread_id)
    }

    /// Gets a mutable reference to a thread by ID
    pub fn get_thread_mut(&mut self, thread_id: ThreadId) -> Option<&mut Thread> {
        self.threads.get_mut(&thread_id)
    }

    /// Deletes a thread
    ///
    /// # Errors
    /// Returns an error if the thread file cannot be deleted
    pub fn delete_thread(&mut self, thread_id: ThreadId) -> Result<()> {
        self.threads.remove(&thread_id);

        let path = self.thread_path(thread_id);
        if path.exists() {
            fs::remove_file(&path).map_err(|err| {
                RoutingError::Other(format!("Failed to delete thread file: {err}"))
            })?;
        }

        Ok(())
    }

    /// Archives a thread (hides from main view but keeps data)
    ///
    /// # Errors
    /// Returns an error if the thread doesn't exist or cannot be saved
    pub fn archive_thread(&mut self, thread_id: ThreadId) -> Result<()> {
        // Get the thread and modify it
        let mut thread = self
            .threads
            .get(&thread_id)
            .ok_or_else(|| RoutingError::Other(format!("Thread {thread_id} not found")))?
            .clone();

        thread.archived = true;
        self.save_thread(&thread)?;

        Ok(())
    }

    /// Unarchives a thread
    ///
    /// # Errors
    /// Returns an error if the thread doesn't exist or cannot be saved
    pub fn unarchive_thread(&mut self, thread_id: ThreadId) -> Result<()> {
        // Get the thread and modify it
        let mut thread = self
            .threads
            .get(&thread_id)
            .ok_or_else(|| RoutingError::Other(format!("Thread {thread_id} not found")))?
            .clone();

        thread.archived = false;
        self.save_thread(&thread)?;

        Ok(())
    }

    /// Returns all non-archived threads
    #[must_use]
    pub fn active_threads(&self) -> Vec<&Thread> {
        self.threads
            .values()
            .filter(|thread| !thread.archived)
            .collect()
    }

    /// Returns all archived threads
    #[must_use]
    pub fn archived_threads(&self) -> Vec<&Thread> {
        self.threads
            .values()
            .filter(|thread| thread.archived)
            .collect()
    }

    /// Returns the total number of threads (including archived)
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.threads.len()
    }

    /// Gets the path for a thread file
    fn thread_path(&self, thread_id: ThreadId) -> PathBuf {
        self.storage_path.join(format!("{thread_id}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_deps::tempfile::TempDir;

    /// # Panics
    /// Helper function - panics indicate test environment failure
    fn create_test_store() -> (ThreadStore, TempDir) {
        let temp_dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(err) => panic!("Failed to create temp dir: {err}"),
        };

        let store = match ThreadStore::new(temp_dir.path().to_path_buf()) {
            Ok(store) => store,
            Err(err) => panic!("Failed to create thread store: {err}"),
        };

        (store, temp_dir)
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_create_thread() {
        let (mut store, _temp) = create_test_store();

        let thread1 = store.create_thread("Thread 1".to_owned());
        let thread2 = store.create_thread("Thread 2".to_owned());

        assert_eq!(thread1.name, "Thread 1");
        assert_eq!(thread2.name, "Thread 2");
        assert_ne!(thread1.color, thread2.color); // Different colors
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_save_and_load_thread() {
        let (mut store, _temp) = create_test_store();

        let thread = store.create_thread("Test".to_owned());
        let thread_id = thread.id;

        assert!(store.save_thread(&thread).is_ok(), "Failed to save thread");

        // Create new store to test loading
        let mut new_store = match ThreadStore::new(store.storage_path.clone()) {
            Ok(store) => store,
            Err(err) => panic!("Failed to create new store: {err}"),
        };
        assert!(new_store.load_all().is_ok(), "Failed to load threads");

        let loaded = new_store.get_thread(thread_id);
        assert!(loaded.is_some(), "Expected thread to exist");
        if let Some(loaded) = loaded {
            assert_eq!(loaded.name, "Test");
        }
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_delete_thread() {
        let (mut store, _temp) = create_test_store();

        let thread = store.create_thread("Delete Me".to_owned());
        let thread_id = thread.id;

        assert!(store.save_thread(&thread).is_ok(), "Failed to save thread");
        assert!(
            store.delete_thread(thread_id).is_ok(),
            "Failed to delete thread"
        );

        assert!(store.get_thread(thread_id).is_none());
        assert!(!store.thread_path(thread_id).exists());
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_archive_thread() {
        let (mut store, _temp) = create_test_store();

        let thread = store.create_thread("Archive Test".to_owned());
        let thread_id = thread.id;

        assert!(store.save_thread(&thread).is_ok(), "Failed to save thread");
        assert!(store.archive_thread(thread_id).is_ok(), "Failed to archive");

        let archived = store.get_thread(thread_id);
        assert!(archived.is_some(), "Thread not found");
        if let Some(archived) = archived {
            assert!(archived.archived);
        }

        assert_eq!(store.active_threads().len(), 0);
        assert_eq!(store.archived_threads().len(), 1);
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_unarchive_thread() {
        let (mut store, _temp) = create_test_store();

        let thread = store.create_thread("Unarchive Test".to_owned());
        let thread_id = thread.id;

        assert!(store.save_thread(&thread).is_ok(), "Failed to save thread");
        assert!(store.archive_thread(thread_id).is_ok(), "Failed to archive");
        assert!(
            store.unarchive_thread(thread_id).is_ok(),
            "Failed to unarchive"
        );

        let unarchived = store.get_thread(thread_id);
        assert!(unarchived.is_some(), "Thread not found");
        if let Some(unarchived) = unarchived {
            assert!(!unarchived.archived);
        }

        assert_eq!(store.active_threads().len(), 1);
        assert_eq!(store.archived_threads().len(), 0);
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_create_branch() {
        let (mut store, _temp) = create_test_store();

        let parent = store.create_thread("Parent".to_owned());
        let parent_id = parent.id;
        assert!(store.save_thread(&parent).is_ok(), "Failed to save parent");

        let msg_id = MessageId::default();
        let branch_result = store.create_branch("Branch".to_owned(), parent_id, msg_id);
        assert!(branch_result.is_ok(), "Failed to create branch");
        let branch = match branch_result {
            Ok(branch) => branch,
            Err(err) => panic!("Failed to create branch: {err}"),
        };

        assert!(branch.parent_thread.is_some());
        if let Some(branch_point) = branch.parent_thread {
            assert_eq!(branch_point.thread_id, parent_id);
        }
    }

    /// # Panics
    /// Test function - panics indicate test failure
    #[test]
    fn test_color_cycling() {
        let (mut store, _temp) = create_test_store();

        let colors: Vec<_> = (0..7)
            .map(|_| store.create_thread("Test".to_owned()).color)
            .collect();

        // First 6 should be different
        for first_color in 0..6 {
            for second_color in (first_color + 1)..6 {
                assert_ne!(colors[first_color], colors[second_color]);
            }
        }

        // 7th should be same as 1st (wraps around)
        assert_eq!(colors[0], colors[6]);
    }
}
