use std::ops::Deref;

use crate::code_lock::{check_new_key, LOCKS};

#[doc(hidden)]
pub fn local_parallel_core_with_return<E>(
    name: &str,
    function: fn() -> Result<(), E>,
) -> Result<(), E> {
    check_new_key(name);

    let unlock = LOCKS.read_recursive();
    unlock.deref()[name].start_parallel();
    let ret = function();
    unlock.deref()[name].end_parallel();
    ret
}

#[doc(hidden)]
pub fn local_parallel_core(name: &str, function: fn()) {
    check_new_key(name);

    let unlock = LOCKS.read_recursive();
    unlock.deref()[name].start_parallel();
    function();
    unlock.deref()[name].end_parallel();
}

#[doc(hidden)]
pub async fn local_async_parallel_core_with_return<E>(
    name: &str,
    fut: impl std::future::Future<Output = Result<(), E>>,
) -> Result<(), E> {
    check_new_key(name);

    let unlock = LOCKS.read_recursive();
    unlock.deref()[name].start_parallel();
    let ret = fut.await;
    unlock.deref()[name].end_parallel();
    ret
}

#[doc(hidden)]
pub async fn local_async_parallel_core(name: &str, fut: impl std::future::Future<Output = ()>) {
    check_new_key(name);

    let unlock = LOCKS.read_recursive();
    unlock.deref()[name].start_parallel();
    fut.await;
    unlock.deref()[name].end_parallel();
}
