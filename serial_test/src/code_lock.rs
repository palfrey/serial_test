use crate::rwlock::{Locks, MutexGuardWrapper};
use lazy_static::lazy_static;
use parking_lot::{Mutex, RwLock};
use std::{
    cell::RefCell,
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{atomic::AtomicU32, Arc},
    time::Duration,
};

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
}

lazy_static! {
    pub(crate) static ref LOCKS: Arc<RwLock<HashMap<String, UniqueReentrantMutex>>> =
        Arc::new(RwLock::new(HashMap::new()));
    static ref MAX_WAIT: Arc<Mutex<RefCell<Duration>>> =
        Arc::new(Mutex::new(RefCell::new(Duration::from_secs(60))));
    static ref MUTEX_ID: Arc<AtomicU32> = Arc::new(AtomicU32::new(1));
}

impl Default for UniqueReentrantMutex {
    fn default() -> Self {
        Self {
            locks: Locks::new(),
            id: MUTEX_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        }
    }
}

/// Sets the maximum amount of time the serial locks will wait to unlock
/// By default, this is set to 60 seconds, which is almost always much longer than is needed
/// This is deliberately set high to try and avoid situations where we accidentally hit the limits
/// but is set at all so we can timeout rather than hanging forever.
///
/// However, sometimes if you've got a *lot* of serial tests it might theoretically not be enough,
/// hence this method.
pub fn set_max_wait(max_wait: Duration) {
    MAX_WAIT.lock().replace(max_wait);
}

pub(crate) fn wait_duration() -> Duration {
    *MAX_WAIT.lock().borrow()
}

pub(crate) fn check_new_key(name: &str) {
    // Check if a new key is needed. Just need a read lock, which can be done in sync with everyone else
    let new_key = {
        let unlock = LOCKS
            .try_read_recursive_for(wait_duration())
            .expect("read lock didn't work");
        !unlock.deref().contains_key(name)
    };
    if new_key {
        // This is the rare path, which avoids the multi-writer situation mostly
        let mut lock = LOCKS
            .try_write_for(wait_duration())
            .expect("write lock didn't work");

        lock.deref_mut().entry(name.to_string()).or_default();
    }
}
