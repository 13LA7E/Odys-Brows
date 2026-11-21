//! A private DevTools channel to Chromium itself, over two anonymous pipes (no network port).
//!
//! It is used for browser-level commands that no CEF call offers: loading and removing extensions
//! while the browser runs (`Extensions.loadUnpacked`, `Extensions.uninstall`). Chromium reads
//! commands from one pipe and writes answers to the other (`--remote-debugging-pipe` with
//! `--remote-debugging-io-pipes=<read>,<write>`, messages end with a NUL byte). Only this
//! process holds the pipe ends, so no other program and no web page can use it.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};

type Callback = Box<dyn FnOnce(Result<Value, String>) + Send>;

static PENDING: Mutex<Option<HashMap<i64, Callback>>> = Mutex::new(None);
static NEXT_ID: AtomicI64 = AtomicI64::new(1);
static WRITE_END: OnceLock<usize> = OnceLock::new();

#[cfg(windows)]
mod sys {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn CreatePipe(read: *mut isize, write: *mut isize, attrs: *const u8, size: u32) -> i32;
        pub fn ReadFile(h: isize, buf: *mut u8, n: u32, got: *mut u32, ov: *mut u8) -> i32;
        pub fn WriteFile(h: isize, buf: *const u8, n: u32, put: *mut u32, ov: *mut u8) -> i32;
    }
}

/// Make the pipes and start reading answers. Returns the value for `--remote-debugging-io-pipes`.
/// Call once, before CEF starts.
pub fn setup() -> Option<String> {
    #[cfg(windows)]
    {
        use std::ptr::null;
        let (mut cmd_read, mut cmd_write, mut ans_read, mut ans_write) = (0isize, 0isize, 0isize, 0isize);
        unsafe {
            if sys::CreatePipe(&mut cmd_read, &mut cmd_write, null(), 1 << 20) == 0
                || sys::CreatePipe(&mut ans_read, &mut ans_write, null(), 1 << 20) == 0
            {
                return None;
            }
        }
        let _ = WRITE_END.set(cmd_write as usize);
        *PENDING.lock().unwrap() = Some(HashMap::new());
        let reader = ans_read as usize;
        std::thread::spawn(move || read_loop(reader as isize));
        Some(format!("{},{}", cmd_read, ans_write))
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
fn read_loop(h: isize) {
    let mut acc: Vec<u8> = Vec::new();
    let mut buf = vec![0u8; 65536];
    loop {
        let mut got = 0u32;
        let ok = unsafe { sys::ReadFile(h, buf.as_mut_ptr(), buf.len() as u32, &mut got, std::ptr::null_mut()) };
        if ok == 0 || got == 0 {
            return;
        }
        acc.extend_from_slice(&buf[..got as usize]);
        while let Some(pos) = acc.iter().position(|b| *b == 0) {
            let msg: Vec<u8> = acc.drain(..=pos).collect();
            let Ok(v) = serde_json::from_slice::<Value>(&msg[..msg.len() - 1]) else {
                continue;
            };
            let Some(id) = v.get("id").and_then(Value::as_i64) else {
                continue; // an event, not an answer
            };
            let cb = PENDING.lock().unwrap().as_mut().and_then(|m| m.remove(&id));
            if let Some(cb) = cb {
                match v.get("error") {
                    Some(e) => cb(Err(e
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("failed")
                        .to_string())),
                    None => cb(Ok(v.get("result").cloned().unwrap_or(Value::Null))),
                }
            }
        }
    }
}

/// Send one command; `cb` runs on the reader thread with the result.
pub fn call(method: &str, params: Value, cb: impl FnOnce(Result<Value, String>) + Send + 'static) {
    #[cfg(windows)]
    {
        let Some(w) = WRITE_END.get().copied() else {
            cb(Err("The browser channel is not available.".into()));
            return;
        };
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        if let Some(m) = PENDING.lock().unwrap().as_mut() {
            m.insert(id, Box::new(cb));
        }
        let mut bytes = json!({ "id": id, "method": method, "params": params })
            .to_string()
            .into_bytes();
        bytes.push(0);
        let mut put = 0u32;
        let ok = unsafe {
            sys::WriteFile(w as isize, bytes.as_ptr(), bytes.len() as u32, &mut put, std::ptr::null_mut())
        };
        if ok == 0
            && let Some(cb) = PENDING.lock().unwrap().as_mut().and_then(|m| m.remove(&id))
        {
            cb(Err("Could not talk to the browser.".into()));
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (method, params, &NEXT_ID, Ordering::Relaxed);
        cb(Err("Not available on this system yet.".into()));
    }
}
