//! # serial_test
//! Helper crate for [serial_test_derive](../serial_test_derive/index.html)

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};
use parking_lot::ReentrantMutex;

lazy_static! {
    static ref LOCKS: Arc<RwLock<HashMap<String, ReentrantMutex<()>>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

/// Helper function for [serial_test_derive::serial](../serial_test_derive/attr.serial.html)
/// ```
/// #[test]
/// fn test_serial_core() {
///     serial_core("some key", || {
///         println!("Bar");
///     });
/// }
/// ```
pub fn serial_core(name: &str, function: fn()) {
    // Check if a new key is needed. Just need a read lock, which can be done in sync with everyone else
    let new_key = {
        let unlock = LOCKS.read().unwrap();
        !unlock.deref().contains_key(name)
    };
    if new_key {
        // This is the rare path, which avoids the multi-writer situation mostly
        LOCKS
            .write()
            .unwrap()
            .deref_mut()
            .insert(name.to_string(), ReentrantMutex::new(()));
    }
    let unlock = LOCKS.read().unwrap();
    // _guard needs to be named to avoid being instant dropped
    let _guard = unlock.deref()[name].lock();
    function();
}

// Re-export #[serial].
#[allow(unused_imports)]
#[doc(hidden)]
pub use serial_test_derive::*;
