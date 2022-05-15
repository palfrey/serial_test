use lazy_static::lazy_static;
use parking_lot::{Mutex, ReentrantMutex, ReentrantMutexGuard, RwLock};
use std::{
    cell::RefCell,
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{atomic::AtomicU32, Arc},
    time::Duration,
};

struct UniqueReentrantMutex {
    mutex: ReentrantMutex<()>,

    // Only actually used for tests
    #[allow(dead_code)]
    id: u32,
}

impl UniqueReentrantMutex {
    fn lock(&self) -> ReentrantMutexGuard<()> {
        self.mutex.lock()
    }
}

lazy_static! {
    static ref LOCKS: Arc<RwLock<HashMap<String, UniqueReentrantMutex>>> =
        Arc::new(RwLock::new(HashMap::new()));
    static ref MAX_WAIT: Arc<Mutex<RefCell<Duration>>> =
        Arc::new(Mutex::new(RefCell::new(Duration::from_secs(60))));
    static ref MUTEX_ID: Arc<AtomicU32> = Arc::new(AtomicU32::new(1));
}

impl Default for UniqueReentrantMutex {
    fn default() -> Self {
        Self {
            mutex: Default::default(),
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

fn wait_duration() -> Duration {
    *MAX_WAIT.lock().borrow()
}

fn check_new_key(name: &str) {
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

#[doc(hidden)]
pub fn local_serial_core_with_return<E>(
    name: &str,
    function: fn() -> Result<(), E>,
) -> Result<(), E> {
    check_new_key(name);

    let unlock = LOCKS.read_recursive();
    // _guard needs to be named to avoid being instant dropped
    let _guard = unlock.deref()[name].lock();
    function()
}

#[doc(hidden)]
pub fn local_serial_core(name: &str, function: fn()) {
    check_new_key(name);

    let unlock = LOCKS.read_recursive();
    // _guard needs to be named to avoid being instant dropped
    let _guard = unlock.deref()[name].lock();
    function();
}

#[doc(hidden)]
pub async fn local_async_serial_core_with_return<E>(
    name: &str,
    fut: impl std::future::Future<Output = Result<(), E>>,
) -> Result<(), E> {
    check_new_key(name);

    let unlock = LOCKS.read_recursive();
    // _guard needs to be named to avoid being instant dropped
    let _guard = unlock.deref()[name].lock();
    fut.await
}

#[doc(hidden)]
pub async fn local_async_serial_core(name: &str, fut: impl std::future::Future<Output = ()>) {
    check_new_key(name);

    let unlock = LOCKS.read_recursive();
    // _guard needs to be named to avoid being instant dropped
    let _guard = unlock.deref()[name].lock();
    fut.await;
}

#[cfg(test)]
mod tests {
    use super::{check_new_key, wait_duration, LOCKS};
    use itertools::Itertools;
    use parking_lot::RwLock;
    use std::{
        ops::Deref,
        sync::{Arc, Barrier},
        thread,
    };

    #[test]
    fn test_hammer_check_new_key() {
        let ptrs = Arc::new(RwLock::new(Vec::new()));
        let mut threads = Vec::new();

        let count = 100;
        let barrier = Arc::new(Barrier::new(count));

        for _ in 0..count {
            let local_locks = LOCKS.clone();
            let local_ptrs = ptrs.clone();
            let c = barrier.clone();
            threads.push(thread::spawn(move || {
                c.wait();
                check_new_key("foo");
                {
                    let unlock = local_locks
                        .try_read_recursive_for(wait_duration())
                        .expect("read lock didn't work");
                    let mutex = unlock.deref().get("foo").unwrap();

                    let mut ptr_guard = local_ptrs
                        .try_write_for(wait_duration())
                        .expect("write lock didn't work");
                    ptr_guard.push(mutex.id);
                }

                c.wait();
            }));
        }
        for thread in threads {
            thread.join().expect("thread join worked");
        }
        let ptrs_read_lock = ptrs
            .try_read_recursive_for(wait_duration())
            .expect("ptrs read work");
        assert_eq!(ptrs_read_lock.len(), count);
        println!("{:?}", ptrs_read_lock);
        assert_eq!(ptrs_read_lock.iter().unique().count(), 1);
    }
}
