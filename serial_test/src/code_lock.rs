use crate::rwlock::{Locks, MutexGuardWrapper};
use once_cell::sync::OnceCell;
use scc::{hash_map::Entry, HashMap};
use std::sync::atomic::AtomicU32;

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
pub(crate) fn global_locks() -> &'static HashMap<String, UniqueReentrantMutex> {
    #[cfg(feature = "test_logging")]
    let _ = env_logger::builder().try_init();
    static LOCKS: OnceCell<HashMap<String, UniqueReentrantMutex>> = OnceCell::new();
    LOCKS.get_or_init(HashMap::new)
}

static MUTEX_ID: AtomicU32 = AtomicU32::new(1);

impl UniqueReentrantMutex {
    fn new_mutex(name: &str) -> Self {
        Self {
            locks: Locks::new(name),
            id: MUTEX_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        }
    }
}

pub(crate) fn check_new_key(name: &str) {
    // Check if a new key is needed. Just need a read lock, which can be done in sync with everyone else
    if global_locks().contains(name) {
        return;
    };

    // This is the rare path, which avoids the multi-writer situation mostly
    let entry = global_locks().entry(name.to_owned());
    match entry {
        Entry::Occupied(o) => o,
        Entry::Vacant(v) => v.insert_entry(UniqueReentrantMutex::new_mutex(name)),
    };
}
