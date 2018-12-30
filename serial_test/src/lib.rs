use antidote;
use std::collections::HashMap;
use std::cell::RefCell;

thread_local!(static LOCKS: RefCell<HashMap<String, antidote::Mutex<()>>> = RefCell::new(HashMap::new()));

pub fn serial_core(name: &str, function: fn()) {
    LOCKS.with(|ll| {
        let mut local_lock = ll.borrow_mut();
        if !local_lock.contains_key(name) {
            local_lock.insert(name.to_string(), antidote::Mutex::new(()));
        }
        let _guard = local_lock[name].lock();
        function();
    });
}