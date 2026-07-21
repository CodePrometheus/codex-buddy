use std::collections::BTreeSet;
use std::ffi::{OsString, c_int, c_uint, c_void};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use crate::paths::Paths;
use crate::registry::Registry;

const CTL_KERN: c_int = 1;
const KERN_ARGMAX: c_int = 8;
const KERN_PROCARGS2: c_int = 49;

// All from libSystem, which every Rust binary links on macOS.
unsafe extern "C" {
    fn proc_listallpids(buffer: *mut c_void, buffersize: c_int) -> c_int;
    fn proc_name(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
    fn sysctl(
        name: *mut c_int,
        namelen: c_uint,
        oldp: *mut c_void,
        oldlenp: *mut usize,
        newp: *mut c_void,
        newlen: usize,
    ) -> c_int;
}

/// Aliases with a live `codex` process attributed to their CODEX_HOME.
///
/// Enumerates pids (libproc), keeps processes named `codex`, and reads each one's exec-time
/// environment via `sysctl(KERN_PROCARGS2)` — the same kernel interface `ps -Eww` uses; it is
/// readable for same-user processes with no subprocess, elevated privilege, or TCC prompt.
/// A process's home is its CODEX_HOME, else `$HOME/.codex`; a home equal to `~/.codex` counts
/// as the active account, since `~/.codex/auth.json` symlinks to that account's credential.
/// Anything unreadable is skipped and any failure yields an empty set; this only feeds an
/// optional UI indicator, never switching or data integrity.
pub fn running_accounts(paths: &Paths, reg: &Registry) -> BTreeSet<String> {
    let dirs: Vec<(&str, PathBuf)> = reg
        .accounts
        .iter()
        .map(|rec| (rec.alias.as_str(), canonical(&paths.account_dir(&rec.dir))))
        .collect();
    let default_home = canonical(paths.codex_home());

    let mut running = BTreeSet::new();
    for pid in all_pids() {
        if process_name(pid).as_deref() != Some("codex") {
            continue;
        }
        let Some(blob) = process_args(pid) else {
            continue;
        };
        let home = match env_var(&blob, "CODEX_HOME") {
            Some(dir) => PathBuf::from(dir),
            None => match env_var(&blob, "HOME") {
                Some(home) => PathBuf::from(home).join(".codex"),
                None => continue,
            },
        };
        let home = canonical(&home);
        if home == default_home {
            if let Some(active) = &reg.active_account {
                running.insert(active.clone());
            }
        } else if let Some((alias, _)) = dirs.iter().find(|(_, dir)| *dir == home) {
            running.insert((*alias).to_string());
        }
    }
    running
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Every pid on the system, or empty on failure.
fn all_pids() -> Vec<c_int> {
    // SAFETY: a null buffer asks libproc for the current pid count.
    let count = unsafe { proc_listallpids(std::ptr::null_mut(), 0) };
    if count <= 0 {
        return Vec::new();
    }
    // Headroom for processes spawned between the two calls.
    let mut pids: Vec<c_int> = vec![0; count as usize + 16];
    let bytes = (pids.len() * size_of::<c_int>()) as c_int;
    // SAFETY: the buffer is valid for `bytes` bytes; libproc fills at most that many.
    let filled = unsafe { proc_listallpids(pids.as_mut_ptr().cast(), bytes) };
    if filled <= 0 {
        return Vec::new();
    }
    pids.truncate(filled as usize);
    pids.retain(|&pid| pid > 0);
    pids
}

/// The short process name (`ps`'s COMM column), if readable.
fn process_name(pid: c_int) -> Option<String> {
    let mut buf = [0u8; 64];
    // SAFETY: the buffer is valid for its full length; libproc fills at most that many bytes.
    let len = unsafe { proc_name(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
    if len <= 0 {
        return None;
    }
    Some(String::from_utf8_lossy(&buf[..len as usize]).into_owned())
}

/// The raw KERN_PROCARGS2 blob (argc + exec path + argv + environment) for `pid`.
fn process_args(pid: c_int) -> Option<Vec<u8>> {
    let mut mib = [CTL_KERN, KERN_PROCARGS2, pid];
    let mut size = arg_max();
    let mut blob = vec![0u8; size];
    // SAFETY: `size` matches the buffer length; the kernel shrinks it to the bytes written.
    let rc = unsafe {
        sysctl(
            mib.as_mut_ptr(),
            mib.len() as c_uint,
            blob.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        return None;
    }
    blob.truncate(size);
    Some(blob)
}

/// The kernel's maximum args-blob size (KERN_ARGMAX), with a 1 MiB fallback.
fn arg_max() -> usize {
    let mut mib = [CTL_KERN, KERN_ARGMAX];
    let mut value: c_int = 0;
    let mut size = size_of::<c_int>();
    // SAFETY: `value` is a c_int and `size` is its exact byte length.
    let rc = unsafe {
        sysctl(
            mib.as_mut_ptr(),
            mib.len() as c_uint,
            (&raw mut value).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc == 0 && value > 0 {
        value as usize
    } else {
        1 << 20
    }
}

/// The value of `key` in a KERN_PROCARGS2 blob's environment section.
///
/// Blob layout: argc as a native-endian i32, the executable path, NUL padding, `argc`
/// NUL-terminated argv strings, then NUL-terminated `KEY=value` environment strings.
fn env_var(blob: &[u8], key: &str) -> Option<OsString> {
    let argc = usize::try_from(i32::from_ne_bytes(blob.get(..4)?.try_into().ok()?)).ok()?;
    let mut rest = &blob[4..];
    let path_end = rest.iter().position(|&b| b == 0)?;
    rest = &rest[path_end..];
    let argv_start = rest.iter().position(|&b| b != 0)?;
    rest = &rest[argv_start..];
    // Skip argv so an argument like `CODEX_HOME=…` can't be mistaken for the environment.
    for _ in 0..argc {
        let end = rest.iter().position(|&b| b == 0)?;
        rest = &rest[end + 1..];
    }
    let prefix = format!("{key}=");
    for entry in rest.split(|&b| b == 0) {
        if entry.is_empty() {
            break;
        }
        if let Some(value) = entry.strip_prefix(prefix.as_bytes()) {
            return Some(OsString::from_vec(value.to_vec()));
        }
    }
    None
}

#[cfg(test)]
mod tests;
