use crate::rwlock::{Locks, MutexGuardWrapper};
use lazy_static::lazy_static;
#[cfg(feature = "logging")]
use log::debug;
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{atomic::AtomicU32, Arc},
    time::{Duration, Instant},
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

    #[cfg(test)]
    pub fn parallel_count(&self) -> u32 {
        self.locks.parallel_count()
    }

    #[cfg(test)]
    pub fn is_locked(&self) -> bool {
        self.locks.is_locked()
    }
}

lazy_static! {
    pub(crate) static ref LOCKS: Arc<RwLock<HashMap<String, UniqueReentrantMutex>>> =
        Arc::new(RwLock::new(HashMap::new()));
    static ref MUTEX_ID: Arc<AtomicU32> = Arc::new(AtomicU32::new(1));
}

#[cfg(feature = "timeout")]
lazy_static! {
    static ref MAX_WAIT: Arc<RwLock<Duration>> = Arc::new(RwLock::new(Duration::from_secs(60)));
}

impl Default for UniqueReentrantMutex {
    fn default() -> Self {
        Self {
            locks: Locks::new(),
            id: MUTEX_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        }
    }
}

/// Sets the maximum amount of time the serial locks will wait to unlock.
/// By default, this is set to 60 seconds, which is almost always much longer than is needed.
/// This is deliberately set high to try and avoid situations where we accidentally hit the limits
/// but is set at all so we can timeout rather than hanging forever.
///
/// However, sometimes if you've got a *lot* of serial tests it might theoretically not be enough,
/// hence this method.
///
/// This function is only available when the `timeout` feature is enabled.
#[cfg(feature = "timeout")]
pub fn set_max_wait(max_wait: Duration) {
    *MAX_WAIT.write() = max_wait;
}

pub(crate) fn wait_duration() -> Duration {
    *MAX_WAIT.read()
}

pub(crate) fn check_new_key(name: &str) {
    let start = Instant::now();
    loop {
        #[cfg(all(feature = "logging", feature = "timeout"))]
        {
            let duration = start.elapsed();
            debug!("Waiting for '{}' {:?}", name, duration);
        }
        // Check if a new key is needed. Just need a read lock, which can be done in sync with everyone else
        let try_unlock = LOCKS.try_read_recursive_for(Duration::from_secs(1));
        if let Some(unlock) = try_unlock {
            if unlock.deref().contains_key(name) {
                return;
            }
            drop(unlock); // so that we don't hold the read lock and so the writer can maybe succeed
        } else {
            continue; // wasn't able to get read lock
        }

        // This is the rare path, which avoids the multi-writer situation mostly
        let try_lock = LOCKS.try_write_for(Duration::from_secs(1));

        if let Some(mut lock) = try_lock {
            lock.deref_mut().entry(name.to_string()).or_default();
            return;
        }

        // If the try_lock fails, then go around the loop again
        // Odds are another test was also locking on the write and has now written the key

        #[cfg(feature = "timeout")]
        {
            let duration = start.elapsed();
            if duration > wait_duration() {
                panic!("Timeout waiting for '{}' {:?}", name, duration);
            }
        }
    }
}
