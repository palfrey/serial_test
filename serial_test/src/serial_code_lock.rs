#![allow(clippy::await_holding_lock)]

use crate::code_lock::{check_new_key, LOCKS};
use std::ops::Deref;

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
#[allow(clippy::print_stdout)]
mod tests {
    use super::local_serial_core;
    use crate::code_lock::{check_new_key, wait_duration, LOCKS};
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

    #[test]
    fn unlock_on_assert() {
        let _ = std::panic::catch_unwind(|| {
            local_serial_core("assert", || {
                assert!(false);
            })
        });
        let unlock = LOCKS.read_recursive();
        assert!(!unlock.deref()["assert"].is_locked());
    }
}
