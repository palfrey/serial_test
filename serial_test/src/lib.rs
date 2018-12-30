use antidote::{Mutex, RwLock};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

lazy_static! {
    static ref LOCKS: Arc<RwLock<HashMap<String, Mutex<()>>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

pub fn serial_core(name: &str, function: fn()) {
    // Check if a new key is needed. Just need a read lock, which can be done in sync with everyone else
    let new_key = {
        let unlock = LOCKS.read();
        !unlock.deref().contains_key(name)
    };
    if new_key {
        // This is the rare path, which avoids the multi-writer situation mostly
        LOCKS
            .write()
            .deref_mut()
            .insert(name.to_string(), Mutex::new(()));
    }
    let unlock = LOCKS.read();
    // _guard needs to be named to avoid being instant dropped
    let _guard = unlock.deref()[name].lock();
    function();
}
