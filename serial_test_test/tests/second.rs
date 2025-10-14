#[cfg(feature = "file_locks")]
use serial_test::file_serial;

#[cfg(feature = "file_locks")]
#[test]
#[file_serial(path => "./relative")]
fn test_file_relative_2() {
    use serial_test_test::{fs_test_fn, RELATIVE_FS};

    fs_test_fn(2, RELATIVE_FS);
}
