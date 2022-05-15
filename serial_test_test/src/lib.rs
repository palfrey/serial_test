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
use std::convert::TryInto;
use std::env;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Barrier;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

lazy_static! {
    static ref LOCK: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    static ref THREAD_ORDERINGS: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(Vec::new()));
    static ref PARALLEL_BARRIER: Arc<Barrier> = Arc::new(Barrier::new(3));
}

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

pub fn test_fn(count: usize) {
    init();
    println!("Start {}", count);
    LOCK.store(count, Ordering::Relaxed);
    thread::sleep(Duration::from_millis(1000 * (count as u64)));
    println!("End {}", count);
    assert_eq!(LOCK.load(Ordering::Relaxed), count);
}

pub fn fs_test_fn(count: usize) {
    init();
    println!("Start {}", count);
    let mut pathbuf = env::temp_dir();
    pathbuf.push("serial-test-test");
    fs::write(pathbuf.as_path(), count.to_ne_bytes()).unwrap();
    thread::sleep(Duration::from_millis(1000 * (count as u64)));
    println!("End {}", count);

    let loaded = fs::read(pathbuf.as_path())
        .map(|bytes| usize::from_ne_bytes(bytes.as_slice().try_into().unwrap()))
        .unwrap();
    assert_eq!(loaded, count);
}

#[cfg(test)]
mod tests {
    use super::{init, test_fn, PARALLEL_BARRIER, THREAD_ORDERINGS};
    use serial_test::{parallel, serial};

    #[cfg(feature = "file_locks")]
    use super::fs_test_fn;
    #[cfg(feature = "file_locks")]
    use serial_test::file_serial;

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
    #[parallel(ordering_key)]
    fn serial_with_parallel_key_1() {
        let count = THREAD_ORDERINGS.lock().unwrap().len();
        // Can't guarantee before or after the parallels
        assert!(count == 0 || count == 3, "count = {}", count);
    }

    #[test]
    #[parallel(ordering_key)]
    fn parallel_with_key_1() {
        PARALLEL_BARRIER.wait();
        THREAD_ORDERINGS.lock().unwrap().push(false);
    }

    #[test]
    #[parallel(ordering_key)]
    fn parallel_with_key_2() {
        PARALLEL_BARRIER.wait();
        THREAD_ORDERINGS.lock().unwrap().push(false);
    }

    #[test]
    #[parallel(ordering_key)]
    fn parallel_with_key_3() {
        PARALLEL_BARRIER.wait();
        THREAD_ORDERINGS.lock().unwrap().push(false);
    }

    #[test]
    #[parallel(ordering_key)]
    fn serial_with_parallel_key_2() {
        let count = THREAD_ORDERINGS.lock().unwrap().len();
        // Can't guarantee before or after the parallels
        assert!(count == 0 || count == 3, "count = {}", count);
    }
}
