use fslock::LockFile;
use std::{env, fs, process, str::from_utf8};

struct Lock {
    lockfile: LockFile,
    needs_unlock: bool,
}

impl Lock {
    fn unlock(self: &mut Lock) {
        if self.needs_unlock {
            self.lockfile.unlock().unwrap();
            println!("Unlock");
        }
    }
}

fn do_lock(path: &str) -> Lock {
    let mut lockfile = LockFile::open(path).unwrap();
    println!("Waiting on {:?}", path);
    let pid_str = format!("{}", process::id());
    let can_lock = lockfile.try_lock().unwrap();
    if !can_lock {
        let raw_file = fs::read(path).unwrap();
        let get_pid = from_utf8(&raw_file).unwrap();
        if get_pid != pid_str {
            lockfile.lock().unwrap();
            fs::write(path, pid_str).unwrap();
        }
    } else {
        println!("Got lock for {:?}", path);
        fs::write(path, pid_str).unwrap();
    }
    Lock {
        lockfile,
        needs_unlock: can_lock,
    }
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

#[test]
fn test_serial() {
    fs_serial_core("test", None, || {
        fs_serial_core("test", None, || {});
    });
}
