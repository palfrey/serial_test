#![allow(clippy::await_holding_lock)]

use crate::code_lock::{check_new_key, LOCKS};

#[doc(hidden)]
macro_rules! core_internal {
    ($fn: ident, $names: ident, $path: ident, $function: ident, $block: expr) => {{
        let name = $names.first().expect("names length > 0").to_owned();
        check_new_key(name);

        let unlock = LOCKS.get(name).expect("key to be set");
        // _guard needs to be named to avoid being instant dropped
        let _guard = unlock.lock();

        if $names.len() > 1 {
            let mut new_names = $names.clone();
            new_names.remove(0);
            $fn(new_names, $path, $function)
        } else {
            $block()
        }
    }};
}

#[doc(hidden)]
pub fn local_serial_core_with_return<E>(
    names: Vec<&str>,
    path: Option<String>,
    function: fn() -> Result<(), E>,
) -> Result<(), E> {
    core_internal!(local_serial_core_with_return, names, path, function, || {
        function()
    })
}

#[doc(hidden)]
pub fn local_serial_core(names: Vec<&str>, path: Option<&str>, function: fn()) {
    core_internal!(local_serial_core, names, path, function, || {
        function();
    });
}

#[doc(hidden)]
#[cfg(feature = "async")]
pub async fn local_async_serial_core_with_return<E>(
    names: Vec<&str>,
    _path: Option<&str>,
    fut: impl std::future::Future<Output = Result<(), E>> + std::marker::Send,
) -> Result<(), E> {
    let unlocks: Vec<_> = names
        .into_iter()
        .map(|name| {
            check_new_key(name);
            LOCKS.get(name).expect("key to be set")
        })
        .collect();
    let _guards = unlocks.iter().map(|unlock| unlock.lock());
    fut.await
}

#[doc(hidden)]
#[cfg(feature = "async")]
pub async fn local_async_serial_core(
    names: Vec<&str>,
    _path: Option<&str>,
    fut: impl std::future::Future<Output = ()>,
) {
    let name = names.first().expect("names length > 0").to_owned();
    check_new_key(name);

    let unlock = LOCKS.get(name).expect("key to be set");
    // _guard needs to be named to avoid being instant dropped
    let _guard = unlock.lock();

    fut.await;
}

#[cfg(test)]
#[allow(clippy::print_stdout)]
mod tests {
    use super::local_serial_core;
    use crate::code_lock::{check_new_key, LOCKS};
    use itertools::Itertools;
    use parking_lot::RwLock;
    use std::{
        sync::{Arc, Barrier},
        thread,
        time::Duration,
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
                    let unlock = local_locks.get("foo").expect("read didn't work");
                    let mutex = unlock.value();

                    let mut ptr_guard = local_ptrs
                        .try_write_for(Duration::from_secs(1))
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
            .try_read_recursive_for(Duration::from_secs(1))
            .expect("ptrs read work");
        assert_eq!(ptrs_read_lock.len(), count);
        println!("{:?}", ptrs_read_lock);
        assert_eq!(ptrs_read_lock.iter().unique().count(), 1);
    }

    #[test]
    fn unlock_on_assert() {
        let _ = std::panic::catch_unwind(|| {
            local_serial_core(vec!["assert"], None, || {
                assert!(false);
            })
        });
        assert!(!LOCKS.get("assert").unwrap().is_locked());
    }
}
