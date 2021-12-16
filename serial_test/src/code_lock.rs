use lazy_static::lazy_static;
use parking_lot::{ReentrantMutex, RwLock, RwLockUpgradableReadGuard};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

lazy_static! {
    static ref LOCKS: Arc<RwLock<HashMap<String, ReentrantMutex<()>>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

fn check_new_key(name: &str) {
    // Check if a new key is needed. Just need a read lock, which can be done in sync with everyone else
    let new_key = {
        let unlock = LOCKS.read();
        !unlock.deref().contains_key(name)
    };
    if new_key {
        // This is the rare path, which avoids the multi-writer situation mostly
        let lock = LOCKS.upgradable_read();
        if lock.deref().contains_key(name) {
            return; // another writer got here between unlock of the read and re-acquire
        }
        let mut upgraded_lock =
            RwLockUpgradableReadGuard::<'_, HashMap<String, ReentrantMutex<()>>>::upgrade(lock);
        upgraded_lock.insert(name.to_string(), ReentrantMutex::new(()));
    }
}

#[doc(hidden)]
pub fn local_serial_core_with_return<E>(
    name: &str,
    function: fn() -> Result<(), E>,
) -> Result<(), E> {
    check_new_key(name);

    let unlock = LOCKS.read();
    // _guard needs to be named to avoid being instant dropped
    let _guard = unlock.deref()[name].lock();
    function()
}

#[doc(hidden)]
pub fn local_serial_core(name: &str, function: fn()) {
    check_new_key(name);

    let unlock = LOCKS.read();
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

    let unlock = LOCKS.read();
    // _guard needs to be named to avoid being instant dropped
    let _guard = unlock.deref()[name].lock();
    fut.await
}

#[doc(hidden)]
pub async fn local_async_serial_core(name: &str, fut: impl std::future::Future<Output = ()>) {
    check_new_key(name);

    let unlock = LOCKS.read();
    // _guard needs to be named to avoid being instant dropped
    let _guard = unlock.deref()[name].lock();
    fut.await;
}

#[cfg(test)]
mod tests {
    use lazy_static::__Deref;

    use super::{check_new_key, LOCKS};
    use std::{
        sync::{Arc, Barrier, RwLock},
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
                let unlock = local_locks.read();
                let mutex = unlock.deref().get("foo").unwrap();
                let _lock = mutex.try_lock();

                let mut ptr_guard = local_ptrs.write().unwrap();
                ptr_guard.push(_lock.is_some());
                drop(ptr_guard);

                c.wait();
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(ptrs.read().unwrap().len(), count);
        println!("{:?}", ptrs.read());
        assert_eq!(ptrs.read().unwrap().iter().filter(|x| **x).count(), 1);
    }
}
