#[cfg(test)]
mod tests {
    use lazy_static::lazy_static;
    use serial_test_derive::serial;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    lazy_static! {
        static ref LOCK: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    }

    #[test]
    #[serial]
    fn test_serial_no_arg() {}

    #[test]
    #[serial(alpha)]
    fn test_serial_1() {
        println!("Start 1");
        LOCK.store(1, Ordering::Relaxed);
        thread::sleep(Duration::from_millis(100));
        println!("End 1");
        assert_eq!(LOCK.load(Ordering::Relaxed), 1);
    }

    #[test]
    #[serial(alpha)]
    fn test_serial_2() {
        println!("Start 2");
        LOCK.store(2, Ordering::Relaxed);
        thread::sleep(Duration::from_millis(200));
        println!("End 2");
        assert_eq!(LOCK.load(Ordering::Relaxed), 2);
    }

    #[test]
    #[serial(alpha)]
    fn test_serial_3() {
        println!("Start 3");
        LOCK.store(3, Ordering::Relaxed);
        thread::sleep(Duration::from_millis(300));
        println!("End 3");
        assert_eq!(LOCK.load(Ordering::Relaxed), 3);
    }

    #[test]
    #[serial]
    fn test_fun() {
        assert_eq!(1 + 2, 3);
    }

    #[test]
    #[serial]
    fn test_reentrant_fun() {
        test_fun();
    }
}
