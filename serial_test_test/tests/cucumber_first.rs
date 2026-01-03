#![cfg(feature="file_locks")]

use cucumber::{World as _, given};
use serial_test::file_serial;

#[derive(Debug, Default, cucumber::World)]
struct World {
}

#[given(expr = "locked file")]
async fn locked_file(_w: &mut World) {
    use serial_test_test::{fs_test_fn, CUCUMBER_FS};
    fs_test_fn(1, CUCUMBER_FS);
}

#[tokio::test]
#[file_serial(path => "./tests/cucumber.lock")]
pub async fn run_cucumber_tests() {
    World::cucumber()
    .fail_fast()
        .run("tests/cucumber.feature")
        .await;
}

