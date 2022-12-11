#![allow(clippy::print_stdout)] // because test code

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

use lazy_static::lazy_static;
use std::{
    convert::TryInto,
    env, fs,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

lazy_static! {
    static ref LOCK: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
}

fn init() {
    let _ = env_logger::builder().is_test(false).try_init();
}

pub fn test_fn(count: usize) {
    init();
    println!("(non-fs) Start {}", count);
    LOCK.store(count, Ordering::Relaxed);
    thread::sleep(Duration::from_millis(1000 * (count as u64)));
    println!("(non-fs) End {}", count);
    assert_eq!(LOCK.load(Ordering::Relaxed), count);
}

pub fn fs_test_fn(count: usize) {
    init();
    println!("(fs) Start {}", count);
    let mut pathbuf = env::temp_dir();
    pathbuf.push("serial-test-test");
    fs::write(pathbuf.as_path(), count.to_ne_bytes()).unwrap();
    thread::sleep(Duration::from_millis(1000 * (count as u64)));
    println!("(fs) End {}", count);

    let loaded = fs::read(pathbuf.as_path())
        .map(|bytes| usize::from_ne_bytes(bytes.as_slice().try_into().unwrap()))
        .unwrap();
    assert_eq!(loaded, count);
}

#[cfg(test)]
mod tests {
    use super::{init, test_fn};
    use lazy_static::lazy_static;
    use parking_lot::Mutex;
    use serial_test::{parallel, serial};
    use std::{
        sync::{Arc, Barrier},
        thread,
        time::Duration,
    };

    lazy_static! {
        static ref THREAD_ORDERINGS: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(Vec::new()));
        static ref FS_THREAD_ORDERINGS: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(Vec::new()));
        static ref PARALLEL_BARRIER: Barrier = Barrier::new(3);
        static ref FS_PARALLEL_BARRIER: Barrier = Barrier::new(3);
    }

    #[cfg(feature = "file_locks")]
    use super::fs_test_fn;
    #[cfg(feature = "file_locks")]
    use serial_test::{file_parallel, file_serial};

    #[test]
    #[serial(timeout_key, timeout_ms = 60000)]
    fn demo_timeout_with_key() {}

    #[test]
    #[serial(timeout_ms = 60000)]
    fn demo_timeout() {}

    #[test]
    #[serial]
    fn test_serial_no_arg() {
        init();
    }

    #[test]
    #[serial(alpha)]
    fn test_serial_1() {
        test_fn(1)
    }

    #[test]
    #[serial(alpha)]
    fn test_serial_2() {
        test_fn(2)
    }

    #[test]
    #[serial(alpha)]
    fn test_serial_3() {
        test_fn(3)
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

    #[tokio::test]
    #[serial]
    async fn test_async_serial_no_arg() {
        init();
    }

    #[actix_rt::test]
    #[serial]
    async fn test_async_serial_no_arg_actix() {
        init();
    }

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
    #[file_serial(test, "/tmp/test")]
    fn test_file_with_path() {}

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
        println!("Waiting barrier 1");
        PARALLEL_BARRIER.wait();
        println!("Waiting lock 1");
        THREAD_ORDERINGS.lock().push(false);
    }

    #[test]
    #[parallel(ordering_key)]
    fn parallel_with_key_2() {
        thread::sleep(Duration::from_secs(2));
        println!("Waiting barrier 2");
        PARALLEL_BARRIER.wait();
        println!("Waiting lock 2");
        THREAD_ORDERINGS.lock().push(false);
    }

    #[test]
    #[parallel(ordering_key)]
    fn parallel_with_key_3() {
        thread::sleep(Duration::from_secs(3));
        println!("Waiting barrier 3");
        PARALLEL_BARRIER.wait();
        println!("Waiting lock 3");
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
        println!("Waiting barrier 1");
        FS_PARALLEL_BARRIER.wait();
        println!("Waiting lock 1");
        FS_THREAD_ORDERINGS.lock().push(false);
    }

    #[cfg(feature = "file_locks")]
    #[test]
    #[file_parallel(ordering_key)]
    fn file_parallel_with_key_2() {
        init();
        thread::sleep(Duration::from_secs(1));
        println!("Waiting barrier 2");
        FS_PARALLEL_BARRIER.wait();
        println!("Waiting lock 2");
        FS_THREAD_ORDERINGS.lock().push(false);
    }

    #[cfg(feature = "file_locks")]
    #[test]
    #[file_parallel(ordering_key)]
    fn file_parallel_with_key_3() {
        init();
        thread::sleep(Duration::from_secs(1));
        println!("Waiting barrier 3");
        FS_PARALLEL_BARRIER.wait();
        println!("Waiting lock 3");
        FS_THREAD_ORDERINGS.lock().push(false);
    }

    #[cfg(feature = "file_locks")]
    #[test]
    #[file_parallel]
    fn file_parallel_with_return() -> Result<(), ()> {
        init();
        Ok(())
    }

    #[cfg(feature = "file_locks")]
    #[tokio::test]
    #[file_parallel]
    async fn file_parallel_with_async_return() -> Result<(), ()> {
        Ok(())
    }

    #[cfg(feature = "file_locks")]
    #[tokio::test]
    #[file_parallel]
    async fn file_parallel_with_async() {
        init();
    }

    // Note, not actually a test as such, just a "can you wrap serial functions" compile-time check
    #[serial]
    async fn async_attribute_works() {
    }
    #[serial]
    async fn async_attribute_works_with_return() -> Result<(), ()> {
        Ok(())
    }
}
