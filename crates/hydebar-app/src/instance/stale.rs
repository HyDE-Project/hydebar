//! Reading the recorded owner and telling a live one from a dead one.
//!
//! The kernel already drops a dead owner's `flock`, so these helpers never
//! decide whether the slot is free — they only turn the process id left in
//! the lock file into an accurate description of who, if anyone, still holds
//! it, so the takeover signals a live incumbent and stays silent about a
//! stale one.

use std::{fs::File, io, io::Read, path::Path};

/// Reads the process id recorded in the lock file, if it holds a readable one.
pub(super) fn read_owner(path: &Path) -> Option<i32> {
    let mut contents = String::new();
    File::open(path).ok()?.read_to_string(&mut contents).ok()?;

    contents.trim().parse::<i32>().ok()
}

/// Reports whether a process with this id still exists.
pub(super) fn process_is_alive(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }

    if unsafe { libc::kill(pid, 0) } == 0 {
        return true;
    }

    io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}
