use crate::rwlock::{Locks, MutexGuardWrapper};
use dashmap::{try_result::TryResult, DashMap};
#[cfg(feature = "logging")]
use log::debug;
use once_cell::sync::OnceCell;
use std::sync::atomic::AtomicU32;
#[cfg(feature = "logging")]
use std::time::Instant;

#[derive(Clone)]
pub(crate) struct UniqueReentrantMutex {
    locks: Locks,

    // Only actually used for tests
    #[allow(dead_code)]
    pub(crate) id: u32,
}

impl UniqueReentrantMutex {
    pub(crate) fn lock(&self) -> MutexGuardWrapper {
        self.locks.serial()
    }

    pub(crate) fn start_parallel(&self) {
        self.locks.start_parallel();
    }

    pub(crate) fn end_parallel(&self) {
        self.locks.end_parallel();
    }

    #[cfg(test)]
    pub fn parallel_count(&self) -> u32 {
        self.locks.parallel_count()
    }

    #[cfg(test)]
    pub fn is_locked(&self) -> bool {
        self.locks.is_locked()
    }
}

#[inline]
pub(crate) fn global_locks() -> &'static DashMap<String, UniqueReentrantMutex> {
    static LOCKS: OnceCell<DashMap<String, UniqueReentrantMutex>> = OnceCell::new();
    LOCKS.get_or_init(DashMap::new)
}

static MUTEX_ID: AtomicU32 = AtomicU32::new(1);

impl Default for UniqueReentrantMutex {
    fn default() -> Self {
        Self {
            locks: Locks::new(),
            id: MUTEX_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        }
    }
}

pub(crate) fn check_new_key(name: &str) {
    #[cfg(feature = "logging")]
    let start = Instant::now();
    loop {
        #[cfg(feature = "logging")]
        {
            let duration = start.elapsed();
            debug!("Waiting for '{}' {:?}", name, duration);
        }
        // Check if a new key is needed. Just need a read lock, which can be done in sync with everyone else
        match global_locks().try_get(name) {
            TryResult::Present(_) => {
                return;
            }
            TryResult::Locked => {
                continue; // wasn't able to get read lock
            }
            TryResult::Absent => {} // do the write path below
        };

        // This is the rare path, which avoids the multi-writer situation mostly
        let try_entry = global_locks().try_entry(name.to_string());

        if let Some(entry) = try_entry {
            entry.or_default();
            return;
        }

        // If the try_entry fails, then go around the loop again
        // Odds are another test was also locking on the write and has now written the key
    }
}
