use lazy_static::lazy_static;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

lazy_static! {
    static ref LOCK: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
}

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

/// Not inside the cfg(test) block because of https://github.com/rust-lang/rust/issues/45599
/// ```
/// #[macro_use] extern crate serial_test;
/// extern crate serial_test_test;
/// use serial_test_test::{test_fn};
/// // #[serial_test::serial]
/// fn main() {
/// test_fn(4);
/// }
/// ```
/// ```
/// #[macro_use] extern crate serial_test;
/// extern crate serial_test_test;
/// use serial_test_test::{test_fn};
/// // #[serial_test::serial]
/// fn main() {
/// test_fn(5);
/// }
/// ```
/// ```
/// #[macro_use] extern crate serial_test;
/// extern crate serial_test_test;
/// use serial_test_test::{test_fn};
/// // #[serial_test::serial]
/// fn main() {
/// test_fn(6);
/// }
/// ```
pub fn test_fn(count: usize) {
    init();
    println!("Start {}", count);
    LOCK.store(count, Ordering::Relaxed);
    thread::sleep(Duration::from_millis(1000 * (count as u64)));
    println!("End {}", count);
    assert_eq!(LOCK.load(Ordering::Relaxed), count);
}


#[cfg(test)]
mod tests {
    use super::{init, test_fn};
    use serial_test::serial;

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

    #[tokio::test]
    #[serial]
    async fn test_async_can_return() -> Result<(), ()> {
        init();
        Ok(())
    }
}
