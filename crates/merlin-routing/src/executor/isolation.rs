use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{Result, RoutingError, TaskId};

/// Tracks which files are being modified by which tasks
pub struct FileLockManager {
    write_locks: RwLock<HashMap<PathBuf, TaskId>>,
    read_locks: RwLock<HashMap<PathBuf, HashSet<TaskId>>>,
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
    pub async fn acquire_write_locks(
        &self,
        task_id: TaskId,
        files: &[PathBuf],
    ) -> Result<WriteLockGuard> {
        let mut write_locks = self.write_locks.write().await;
        let read_locks = self.read_locks.read().await;
        
        for file in files {
            if let Some(&holder) = write_locks.get(file) {
                if holder != task_id {
                    return Err(RoutingError::FileLockedByTask {
                        file: file.clone(),
                        holder,
                    });
                }
            }
            
            if let Some(readers) = read_locks.get(file) {
                if !readers.is_empty() && !readers.contains(&task_id) {
                    return Err(RoutingError::FileHasActiveReaders {
                        file: file.clone(),
                        readers: readers.len(),
                    });
                }
            }
        }
        
        for file in files {
            write_locks.insert(file.clone(), task_id);
        }
        
        Ok(WriteLockGuard {
            manager: Arc::new(self.clone_manager()),
            task_id,
            files: files.to_vec(),
        })
    }
    
    /// Acquire read lock on files (shared access)
    pub async fn acquire_read_locks(
        &self,
        task_id: TaskId,
        files: &[PathBuf],
    ) -> Result<ReadLockGuard> {
        let write_locks = self.write_locks.read().await;
        let mut read_locks = self.read_locks.write().await;
        
        for file in files {
            if let Some(&holder) = write_locks.get(file) {
                if holder != task_id {
                    return Err(RoutingError::FileLockedByTask {
                        file: file.clone(),
                        holder,
                    });
                }
            }
        }
        
        for file in files {
            read_locks
                .entry(file.clone())
                .or_insert_with(HashSet::new)
                .insert(task_id);
        }
        
        Ok(ReadLockGuard {
            manager: Arc::new(self.clone_manager()),
            task_id,
            files: files.to_vec(),
        })
    }
    
    async fn release_write_locks(&self, task_id: TaskId, files: &[PathBuf]) {
        let mut write_locks = self.write_locks.write().await;
        for file in files {
            if let Some(&holder) = write_locks.get(file) {
                if holder == task_id {
                    write_locks.remove(file);
                }
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
    
    fn clone_manager(&self) -> Self {
        Self {
            write_locks: RwLock::new(HashMap::new()),
            read_locks: RwLock::new(HashMap::new()),
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
        let manager = self.manager.clone();
        let task_id = self.task_id;
        let files = self.files.clone();
        
        tokio::spawn(async move {
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
        let manager = self.manager.clone();
        let task_id = self.task_id;
        let files = self.files.clone();
        
        tokio::spawn(async move {
            manager.release_read_locks(task_id, &files).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write_lock_exclusive() {
        let manager = FileLockManager::new();
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let file = PathBuf::from("test.rs");
        
        let _guard_a = manager.acquire_write_locks(task_a, &[file.clone()]).await.unwrap();
        
        let result = manager.acquire_write_locks(task_b, &[file]).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_read_locks_shared() {
        let manager = FileLockManager::new();
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let file = PathBuf::from("test.rs");
        
        let _guard_a = manager.acquire_read_locks(task_a, &[file.clone()]).await.unwrap();
        let _guard_b = manager.acquire_read_locks(task_b, &[file]).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_write_blocks_read() {
        let manager = FileLockManager::new();
        let task_a = TaskId::new();
        let task_b = TaskId::new();
        let file = PathBuf::from("test.rs");
        
        let _guard_a = manager.acquire_write_locks(task_a, &[file.clone()]).await.unwrap();
        
        let result = manager.acquire_read_locks(task_b, &[file]).await;
        assert!(result.is_err());
    }
}
