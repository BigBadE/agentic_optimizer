use crate::{Result, RoutingError, TaskId};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::spawn;

type WriteLocks = HashMap<PathBuf, TaskId>;
type ReadLocks = HashMap<PathBuf, HashSet<TaskId>>;

/// Tracks which files are being modified by which tasks
pub struct FileLockManager {
    write_locks: RwLock<WriteLocks>,
    read_locks: RwLock<ReadLocks>,
}

impl FileLockManager {
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            write_locks: RwLock::new(HashMap::new()),
            read_locks: RwLock::new(HashMap::new()),
        })
    }

    /// Acquire write lock on files (exclusive access)
    ///
    /// # Errors
    /// Returns an error if any file is already locked by another task or has active readers.
    pub async fn acquire_write_locks(
        self: &Arc<Self>,
        task_id: TaskId,
        files: &[PathBuf],
    ) -> Result<WriteLockGuard> {
        // Check for conflicts first with minimal locking and short-lived guards
        for file in files {
            {
                let write_locks_read = self.write_locks.read().await;
                if let Some(&holder) = write_locks_read.get(file)
                    && holder != task_id
                {
                    return Err(RoutingError::FileLockedByTask {
                        file: file.clone(),
                        holder,
                    });
                }
            }

            {
                let read_locks_read = self.read_locks.read().await;
                if let Some(readers) = read_locks_read.get(file)
                    && !readers.is_empty()
                    && !readers.contains(&task_id)
                {
                    return Err(RoutingError::FileHasActiveReaders {
                        file: file.clone(),
                        readers: readers.len(),
                    });
                }
            }
        }

        {
            // Acquire write lock only when inserting
            let mut write_locks = self.write_locks.write().await;
            for file in files {
                write_locks.insert(file.clone(), task_id);
            }
        }

        Ok(WriteLockGuard {
            manager: Arc::clone(self),
            task_id,
            files: files.to_vec(),
        })
    }

    /// Acquire read lock on files (shared access)
    ///
    /// # Errors
    /// Returns an error if any file currently has an exclusive writer.
    pub async fn acquire_read_locks(
        self: &Arc<Self>,
        task_id: TaskId,
        files: &[PathBuf],
    ) -> Result<ReadLockGuard> {
        {
            // First, check write locks with a read guard
            let write_locks = self.write_locks.read().await;
            for file in files {
                if let Some(&holder) = write_locks.get(file)
                    && holder != task_id
                {
                    return Err(RoutingError::FileLockedByTask {
                        file: file.clone(),
                        holder,
                    });
                }
            }
        }

        {
            // Then insert into read locks with a write guard
            let mut read_locks = self.read_locks.write().await;
            for file in files {
                read_locks
                    .entry(file.clone())
                    .or_insert_with(HashSet::new)
                    .insert(task_id);
            }
        }

        Ok(ReadLockGuard {
            manager: Arc::clone(self),
            task_id,
            files: files.to_vec(),
        })
    }

    async fn release_write_locks(&self, task_id: TaskId, files: &[PathBuf]) {
        let mut write_locks = self.write_locks.write().await;
        for file in files {
            if let Some(&holder) = write_locks.get(file)
                && holder == task_id
            {
                write_locks.remove(file);
            }
        }
    }

    async fn release_read_locks(&self, task_id: TaskId, files: &[PathBuf]) {
        let mut read_locks = self.read_locks.write().await;
        for file in files {
            if let Some(readers) = read_locks.get_mut(file) {
                readers.remove(&task_id);
                if readers.is_empty() {
                    read_locks.remove(file);
                }
            }
        }
    }
}

impl Default for FileLockManager {
    fn default() -> Self {
        Self {
            write_locks: RwLock::new(HashMap::new()),
            read_locks: RwLock::new(HashMap::new()),
        }
    }
}

/// RAII guard for write locks - released on drop
pub struct WriteLockGuard {
    manager: Arc<FileLockManager>,
    task_id: TaskId,
    files: Vec<PathBuf>,
}

impl Drop for WriteLockGuard {
    fn drop(&mut self) {
        let manager = Arc::clone(&self.manager);
        let task_id = self.task_id;
        let files = self.files.clone();

        spawn(async move {
            manager.release_write_locks(task_id, &files).await;
        });
    }
}

/// RAII guard for read locks - released on drop
pub struct ReadLockGuard {
    manager: Arc<FileLockManager>,
    task_id: TaskId,
    files: Vec<PathBuf>,
}

impl Drop for ReadLockGuard {
    fn drop(&mut self) {
        let manager = Arc::clone(&self.manager);
        let task_id = self.task_id;
        let files = self.files.clone();

        spawn(async move {
            manager.release_read_locks(task_id, &files).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::slice::from_ref;

    #[tokio::test]
    /// # Panics
    /// Panics if lock acquisition unexpectedly fails in the test harness.
    async fn test_write_lock_exclusive() {
        let manager = FileLockManager::new();
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let file = PathBuf::from("test.rs");

        let _guard_a = match manager.acquire_write_locks(task_a, from_ref(&file)).await {
            Ok(guard) => guard,
            Err(error) => panic!("failed to acquire write lock: {error}"),
        };

        let result = manager.acquire_write_locks(task_b, &[file]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    /// # Panics
    /// Panics if lock acquisition unexpectedly fails in the test harness.
    async fn test_read_locks_shared() {
        let manager = FileLockManager::new();
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let file = PathBuf::from("test.rs");

        let _guard_a = match manager.acquire_read_locks(task_a, from_ref(&file)).await {
            Ok(guard) => guard,
            Err(error) => panic!("failed to acquire read lock: {error}"),
        };
        let _guard_b = match manager.acquire_read_locks(task_b, from_ref(&file)).await {
            Ok(guard) => guard,
            Err(error) => panic!("failed to acquire read lock: {error}"),
        };
    }

    #[tokio::test]
    /// # Panics
    /// Panics if lock acquisition unexpectedly fails in the test harness.
    async fn test_write_blocks_read() {
        let manager = FileLockManager::new();
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let file = PathBuf::from("test.rs");

        let _guard_a = match manager.acquire_write_locks(task_a, from_ref(&file)).await {
            Ok(guard) => guard,
            Err(error) => panic!("failed to acquire write lock: {error}"),
        };

        let result = manager.acquire_read_locks(task_b, from_ref(&file)).await;
        assert!(result.is_err());
    }
}
