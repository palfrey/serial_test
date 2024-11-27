//! Not inside the cfg(test) block because of <https://github.com/rust-lang/rust/issues/45599>
//! ```
//! #[macro_use] extern crate serial_test;
//! extern crate serial_test_test;
//! use serial_test_test::{fs_test_fn};
//! #[cfg(feature = "file_locks")]
//! #[serial_test::file_serial]
//! fn main() {
//! fs_test_fn(1);
//! }
//! #[cfg(not(feature = "file_locks"))]
//! fn main() {}
//! ```
//! ```
//! #[macro_use] extern crate serial_test;
//! extern crate serial_test_test;
//! use serial_test_test::{fs_test_fn};
//! #[cfg(feature = "file_locks")]
//! #[serial_test::file_serial]
//! fn main() {
//! fs_test_fn(2);
//! }
//! #[cfg(not(feature = "file_locks"))]
//! fn main() {}
//! ```
//! ```
//! #[macro_use] extern crate serial_test;
//! extern crate serial_test_test;
//! use serial_test_test::{fs_test_fn};
//! #[cfg(feature = "file_locks")]
//! #[serial_test::file_serial]
//! fn main() {
//! fs_test_fn(3);
//! }
//! #[cfg(not(feature = "file_locks"))]
//! fn main() {}
//! ```

use log::info;
use once_cell::sync::OnceCell;
use scc::HashMap;
#[cfg(test)]
use serial_test::{parallel, serial};
use std::{
    convert::TryInto,
    env, fs,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Duration,
};

static LOCKS: OnceCell<HashMap<String, AtomicUsize>> = OnceCell::new();

fn init() {
    let _ = env_logger::builder().is_test(false).try_init();
}

pub fn test_fn(key: &str, count: usize) {
    init();
    let local_locks = LOCKS.get_or_init(HashMap::new);
    let entry = local_locks
        .entry(key.to_string())
        .or_insert(AtomicUsize::new(0));
    let local_lock = entry.get();
    info!("(non-fs) Start {}", count);
    local_lock.store(count, Ordering::Relaxed);
    thread::sleep(Duration::from_millis(1000 * (count as u64)));
    info!("(non-fs) End {}", count);
    assert_eq!(local_lock.load(Ordering::Relaxed), count);
}

pub fn fs_test_fn(count: usize) {
    init();
    info!("(fs) Start {}", count);
    let mut pathbuf = env::temp_dir();
    pathbuf.push("serial-test-test");
    fs::write(pathbuf.as_path(), count.to_ne_bytes()).unwrap();
    thread::sleep(Duration::from_millis(1000 * (count as u64)));
    info!("(fs) End {}", count);

    let loaded = fs::read(pathbuf.as_path())
        .map(|bytes| usize::from_ne_bytes(bytes.as_slice().try_into().unwrap()))
        .unwrap();
    assert_eq!(loaded, count);
}

#[cfg(test)]
#[serial]
mod serial_attr_tests {}

#[cfg(test)]
#[parallel]
mod parallel_attr_tests {}

#[cfg(test)]
mod tests {
    use super::{init, test_fn};
    use log::info;
    use once_cell::sync::OnceCell;
    use parking_lot::Mutex;
    use serial_test::{parallel, serial};
    use std::{sync::Barrier, thread, time::Duration};
    #[cfg(feature = "async")]
    use wasm_bindgen_test::wasm_bindgen_test;

    static THREAD_ORDERINGS: Mutex<Vec<bool>> = Mutex::new(Vec::new());

    #[inline]
    fn parallel_barrier() -> &'static Barrier {
        static PARALLEL_BARRIER: OnceCell<Barrier> = OnceCell::new();
        PARALLEL_BARRIER.get_or_init(|| Barrier::new(3))
    }

    #[cfg(feature = "file_locks")]
    static FS_THREAD_ORDERINGS: Mutex<Vec<bool>> = Mutex::new(Vec::new());

    #[cfg(feature = "file_locks")]
    #[inline]
    fn fs_parallel_barrier() -> &'static Barrier {
        static FS_PARALLEL_BARRIER: OnceCell<Barrier> = OnceCell::new();
        FS_PARALLEL_BARRIER.get_or_init(|| Barrier::new(3))
    }

    #[cfg(feature = "file_locks")]
    use super::fs_test_fn;
    #[cfg(feature = "file_locks")]
    use serial_test::{file_parallel, file_serial};

    #[test]
    #[serial]
    fn test_serial_no_arg() {
        init();
    }

    #[test]
    #[serial(one, two)]
    fn test_serial_multi_arg() {
        init();
    }

    #[test]
    #[serial(alpha)]
    fn test_serial_1() {
        test_fn("alpha", 1)
    }

    #[test]
    #[serial(alpha)]
    fn test_serial_2() {
        test_fn("alpha", 2)
    }

    #[test]
    #[serial(alpha)]
    fn test_serial_3() {
        test_fn("alpha", 3)
    }

    #[test]
    #[serial]
    #[ignore]
    fn test_ignore_fun() {
        init();
        assert_eq!(2 + 2, 5);
    }

    #[test]
    #[serial]
    fn test_reentrant_fun() {
        init();
        test_serial_no_arg();
    }

    #[test]
    #[serial]
    #[should_panic]
    fn test_should_panic_fun() {
        init();
        panic!("Testing panic");
    }

    #[test]
    #[serial]
    fn test_can_return() -> Result<(), ()> {
        init();
        Ok(())
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    #[serial]
    async fn test_async_serial_no_arg_tokio_first() {
        init();
    }

    #[cfg(feature = "async")]
    #[serial]
    #[tokio::test]
    async fn test_async_serial_no_arg_serial_first() {
        init();
    }

    #[cfg(feature = "async")]
    #[serial]
    #[actix_rt::test]
    async fn test_async_serial_no_arg_actix_with_serial_firs() {
        init();
    }

    #[cfg(feature = "async")]
    #[actix_rt::test]
    #[serial]
    async fn test_async_serial_no_arg_actix_first() {
        init();
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    #[serial]
    async fn test_async_can_return() -> Result<(), ()> {
        init();
        Ok(())
    }

    #[cfg(feature = "file_locks")]
    #[test]
    #[file_serial]
    fn test_file_1() {
        fs_test_fn(1);
    }

    #[cfg(feature = "file_locks")]
    #[test]
    #[file_serial]
    fn test_file_2() {
        fs_test_fn(2);
    }

    #[cfg(feature = "file_locks")]
    #[test]
    #[file_serial]
    fn test_file_3() {
        fs_test_fn(3);
    }

    #[cfg(all(feature = "file_locks", not(windows)))]
    #[test]
    #[file_serial(test, path => "/tmp/test")]
    fn test_file_with_path() {}

    #[cfg(all(feature = "file_locks", not(windows)))]
    #[test]
    #[file_serial(path => "/tmp/test")]
    fn test_file_with_path_and_no_key() {}

    #[test]
    #[serial(test_key)]
    fn test_with_key() {
        init();
    }

    #[test]
    #[serial(ordering_key)]
    fn serial_with_parallel_key_1() {
        let count = THREAD_ORDERINGS.lock().len();
        // Can't guarantee before or after the parallels
        assert!(count == 0 || count == 3, "count = {}", count);
    }

    #[test]
    #[parallel(ordering_key)]
    fn parallel_with_key_1() {
        thread::sleep(Duration::from_secs(1));
        info!("Waiting barrier 1");
        parallel_barrier().wait();
        info!("Waiting lock 1");
        THREAD_ORDERINGS.lock().push(false);
    }

    #[test]
    #[parallel(ordering_key)]
    fn parallel_with_key_2() {
        thread::sleep(Duration::from_secs(2));
        info!("Waiting barrier 2");
        parallel_barrier().wait();
        info!("Waiting lock 2");
        THREAD_ORDERINGS.lock().push(false);
    }

    #[test]
    #[parallel(ordering_key)]
    fn parallel_with_key_3() {
        thread::sleep(Duration::from_secs(3));
        info!("Waiting barrier 3");
        parallel_barrier().wait();
        info!("Waiting lock 3");
        THREAD_ORDERINGS.lock().push(false);
    }

    #[test]
    #[serial(ordering_key)]
    fn serial_with_parallel_key_2() {
        let count = THREAD_ORDERINGS.lock().len();
        // Can't guarantee before or after the parallels
        assert!(count == 0 || count == 3, "count = {}", count);
    }

    #[cfg(feature = "file_locks")]
    #[test]
    #[file_serial(ordering_key)]
    fn file_serial_with_parallel_key_1() {
        let count = FS_THREAD_ORDERINGS.lock().len();
        // Can't guarantee before or after the parallels
        assert!(count == 0 || count == 3, "count = {}", count);
    }

    #[cfg(feature = "file_locks")]
    #[test]
    #[file_serial(ordering_key)]
    fn file_serial_with_parallel_key_2() {
        let count = FS_THREAD_ORDERINGS.lock().len();
        // Can't guarantee before or after the parallels
        assert!(count == 0 || count == 3, "count = {}", count);
    }

    #[cfg(feature = "file_locks")]
    #[test]
    #[file_parallel(ordering_key)]
    fn file_parallel_with_key_1() {
        init();
        thread::sleep(Duration::from_secs(1));
        info!("Waiting barrier 1");
        fs_parallel_barrier().wait();
        info!("Waiting lock 1");
        FS_THREAD_ORDERINGS.lock().push(false);
    }

    #[cfg(feature = "file_locks")]
    #[test]
    #[file_parallel(ordering_key)]
    fn file_parallel_with_key_2() {
        init();
        thread::sleep(Duration::from_secs(1));
        info!("Waiting barrier 2");
        fs_parallel_barrier().wait();
        info!("Waiting lock 2");
        FS_THREAD_ORDERINGS.lock().push(false);
    }

    #[cfg(feature = "file_locks")]
    #[test]
    #[file_parallel(ordering_key)]
    fn file_parallel_with_key_3() {
        init();
        thread::sleep(Duration::from_secs(1));
        info!("Waiting barrier 3");
        fs_parallel_barrier().wait();
        info!("Waiting lock 3");
        FS_THREAD_ORDERINGS.lock().push(false);
    }

    #[cfg(feature = "file_locks")]
    #[test]
    #[file_parallel]
    fn file_parallel_with_return() -> Result<(), ()> {
        init();
        Ok(())
    }

    #[cfg(all(feature = "file_locks", feature = "async"))]
    #[tokio::test]
    #[file_parallel]
    async fn file_parallel_with_async_return() -> Result<(), ()> {
        Ok(())
    }

    #[cfg(all(feature = "file_locks", feature = "async"))]
    #[tokio::test]
    #[file_parallel]
    async fn file_parallel_with_async() {
        init();
    }

    // Note, not actually a test as such, just a "can you wrap serial functions" compile-time check
    #[cfg(feature = "async")]
    #[serial]
    async fn async_attribute_works() {}

    #[cfg(feature = "async")]
    #[serial]
    async fn async_attribute_works_with_return() -> Result<(), ()> {
        Ok(())
    }

    #[cfg(feature = "async")]
    #[wasm_bindgen_test]
    #[serial]
    async fn wasm_works_first() {}

    #[cfg(feature = "async")]
    #[serial]
    #[wasm_bindgen_test]
    async fn wasm_works_second() {}

    #[tokio::test(flavor = "multi_thread")]
    #[serial(slt)]
    async fn tokio_multi_1() {
        test_fn("tokio", 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial(slt)]
    async fn tokio_multi_2() {
        test_fn("tokio", 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    #[serial(slt)]
    async fn tokio_multi_3() {
        test_fn("tokio", 3);
    }
}
