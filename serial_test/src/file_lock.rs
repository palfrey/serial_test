use fslock::LockFile;
#[cfg(feature = "logging")]
use log::debug;
use std::{env, fs, path::Path};

struct Lock {
    lockfile: LockFile,
}

impl Lock {
    fn unlock(self: &mut Lock) {
        self.lockfile.unlock().unwrap();
        #[cfg(feature = "logging")]
        debug!("Unlock");
    }
}

fn do_lock(path: &str) -> Lock {
    if !Path::new(path).exists() {
        fs::write(path, "").unwrap_or_else(|_| panic!("Lock file path was {:?}", path))
    }
    let mut lockfile = LockFile::open(path).unwrap();
    #[cfg(feature = "logging")]
    debug!("Waiting on {:?}", path);
    lockfile.lock().unwrap();
    #[cfg(feature = "logging")]
    debug!("Locked for {:?}", path);
    Lock { lockfile }
}

fn path_for_name(name: &str) -> String {
    let mut pathbuf = env::temp_dir();
    pathbuf.push(format!("serial-test-{}", name));
    pathbuf.into_os_string().into_string().unwrap()
}

fn make_lock_for_name_and_path(name: &str, path: Option<&str>) -> Lock {
    if let Some(opt_path) = path {
        do_lock(opt_path)
    } else {
        let default_path = path_for_name(name);
        do_lock(&default_path)
    }
}

#[doc(hidden)]
pub fn fs_serial_core(name: &str, path: Option<&str>, function: fn()) {
    let mut lock = make_lock_for_name_and_path(name, path);
    function();
    lock.unlock();
}

#[doc(hidden)]
pub fn fs_serial_core_with_return<E>(
    name: &str,
    path: Option<&str>,
    function: fn() -> Result<(), E>,
) -> Result<(), E> {
    let mut lock = make_lock_for_name_and_path(name, path);
    let ret = function();
    lock.unlock();
    ret
}

#[doc(hidden)]
pub async fn fs_async_serial_core_with_return<E>(
    name: &str,
    path: Option<&str>,
    fut: impl std::future::Future<Output = Result<(), E>>,
) -> Result<(), E> {
    let mut lock = make_lock_for_name_and_path(name, path);
    let ret = fut.await;
    lock.unlock();
    ret
}

#[doc(hidden)]
pub async fn fs_async_serial_core(
    name: &str,
    path: Option<&str>,
    fut: impl std::future::Future<Output = ()>,
) {
    let mut lock = make_lock_for_name_and_path(name, path);
    fut.await;
    lock.unlock();
}

#[cfg(test)]
mod tests {
    use std::panic;

    use fslock::LockFile;

    use super::{fs_serial_core, path_for_name};

    #[test]
    fn test_serial() {
        fs_serial_core("test", None, || {});
    }

    #[test]
    fn unlock_on_assert_sync_without_return() {
        let lock_path = path_for_name("unlock_on_assert_sync_without_return");
        let _ = panic::catch_unwind(|| {
            fs_serial_core("foo", Some(&lock_path), || {
                assert!(false);
            })
        });
        let mut lockfile = LockFile::open(&lock_path).unwrap();
        assert!(lockfile.try_lock().unwrap());
    }
}
