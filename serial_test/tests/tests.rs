use serial_test::local_serial_core;
use serial_test::{parallel, serial};

#[test]
fn test_empty_serial_call() {
    local_serial_core(vec!["beta"], None, || {
        println!("Bar");
    });
}

// Tests for inner_attrs feature with ntest::timeout
#[test]
#[serial(inner_attrs_test, inner_attrs = [ntest::timeout(5000)])]
fn test_serial_with_inner_timeout() {
    // complete instantly
}

/// Test that inner_attrs works with ntest::timeout on a sync parallel test.
#[test]
#[parallel(inner_attrs_test, inner_attrs = [ntest::timeout(5000)])]
fn test_parallel_with_inner_timeout() {
    // complete instantly
}

/// Test serial with inner_attrs and a key
#[test]
#[serial(my_key, inner_attrs = [ntest::timeout(5000)])]
fn test_serial_with_key_and_inner_timeout() {
    // complete instantly
}

/// Test that multiple inner_attrs can be specified
#[test]
#[serial(inner_attrs = [ntest::timeout(5000),inline])]
fn test_serial_with_multiple_inner_attrs() {
    // complete instantly
}

/// Test that timeout actually triggers
#[test]
#[should_panic]
#[serial(inner_attrs = [ntest::timeout(10)])]
fn test_serial_inner_timeout_panics() {
    std::thread::sleep(std::time::Duration::from_secs(1));
}
