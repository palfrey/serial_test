use antidote::Mutex;
use std::collections::HashMap; 
use std::sync::Arc;
use std::ops::DerefMut;
use lazy_static::lazy_static;

lazy_static! {
    static ref LOCKS: Arc<Mutex<HashMap<String, Mutex<()>>>> = Arc::new(Mutex::new(HashMap::new()));
}

pub fn serial_core(name: &str, function: fn()) {
    let mut unlock = LOCKS.lock();
    let local_lock = unlock.deref_mut();
    if !local_lock.contains_key(name) {
        local_lock.insert(name.to_string(), Mutex::new(()));
    }
    let _guard = local_lock[name].lock();
    function();
}