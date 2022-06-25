use crate::file_lock::make_lock_for_name_and_path;

#[doc(hidden)]
pub fn fs_parallel_core(name: &str, path: Option<&str>, function: fn()) {
    make_lock_for_name_and_path(name, path).start_parallel();
    function();
    make_lock_for_name_and_path(name, path).end_parallel();
}
