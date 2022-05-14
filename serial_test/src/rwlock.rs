use std::sync::{Arc, Condvar, Mutex};

use parking_lot::{ReentrantMutex, ReentrantMutexGuard};

struct LockState {
    parallels: u32,
}

struct LockData {
    mutex: Mutex<LockState>,
    serial: ReentrantMutex<()>,
    condvar: Condvar,
}

#[derive(Clone)]
pub(crate) struct Locks {
    arc: Arc<LockData>,
}

pub(crate) struct MutexGuardWrapper<'a> {
    #[allow(dead_code)] // need it around to get dropped
    mutex_guard: ReentrantMutexGuard<'a, ()>,
    locks: Locks,
}

impl<'a> Drop for MutexGuardWrapper<'a> {
    fn drop(&mut self) {
        self.locks.arc.condvar.notify_one();
    }
}

impl Locks {
    pub fn new() -> Locks {
        Locks {
            arc: Arc::new(LockData {
                mutex: Mutex::new(LockState { parallels: 0 }),
                condvar: Condvar::new(),
                serial: Default::default(),
            }),
        }
    }

    pub fn serial(&self) -> MutexGuardWrapper {
        let mut lock_state = self.arc.mutex.lock().unwrap();
        loop {
            // If all the things we want are true, try to lock out serial
            if lock_state.parallels == 0 {
                let possible_serial_lock = self.arc.serial.try_lock();
                if let Some(serial_lock) = possible_serial_lock {
                    return MutexGuardWrapper {
                        mutex_guard: serial_lock,
                        locks: self.clone(),
                    };
                }
            }

            lock_state = self.arc.condvar.wait(lock_state).unwrap();
        }
    }
}
