//! Segmented (multi-connection) downloader, ported from Odyssey in P4-01.
//!
//! When the browser engine initiates a download, we probe the server to check if it supports byte ranges.
//! If it does (HTTP 206) and the file is big enough (>= 8 MiB), this module fetches the file in chunks
//! over several connections instead, writing straight into an `.odypart` file.
//! On success, `.odypart` is atomically renamed to the destination file.
//! On cancel or failure, the `.odypart` file is deleted immediately (no leftover files).

use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::OpenOptions,
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use futures_util::{StreamExt, future::join, future::join_all};
use serde_json::{Value, json};

/// Files smaller than 8 MiB are left to the engine's single-connection download.
pub const MIN_BYTES: u64 = 8 * 1024 * 1024;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

pub type Emit = Arc<dyn Fn(&str, Value) + Send + Sync>;

/// Number of connections to use per file. 1 turns segmented downloads off. Default is 8.
static CONNS: AtomicUsize = AtomicUsize::new(8);

pub fn set_connections(n: usize) {
    CONNS.store(n.clamp(1, 16), Ordering::Relaxed);
}

pub fn connections() -> usize {
    CONNS.load(Ordering::Relaxed)
}

pub fn enabled() -> bool {
    CONNS.load(Ordering::Relaxed) > 1
}

pub const RUN: u8 = 0;
pub const PAUSE: u8 = 1;
pub const CANCEL: u8 = 2;

pub struct Ctl {
    pub state: AtomicU8,
}

fn jobs() -> &'static Mutex<HashMap<String, Arc<Ctl>>> {
    static J: OnceLock<Mutex<HashMap<String, Arc<Ctl>>>> = OnceLock::new();
    J.get_or_init(|| Mutex::new(HashMap::new()))
}

fn taken() -> &'static Mutex<HashSet<String>> {
    static T: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Same spelling the UI uses to match a download to its path.
pub fn path_key(p: &str) -> String {
    p.replace('\\', "/").to_lowercase()
}

/// Key for matching a download by address.
pub fn url_key(u: &str) -> String {
    format!(
        "url:{}",
        reqwest::Url::parse(u)
            .map(|x| x.to_string())
            .unwrap_or_else(|_| u.to_string())
    )
}

/// True once if the engine's own download for this path/address was taken over.
pub fn was_taken(path_key_: &str, url: &str) -> bool {
    let mut t = taken().lock().unwrap();
    let a = !path_key_.is_empty() && t.remove(path_key_);
    let b = t.remove(&url_key(url));
    a || b
}

/// Forget takeover marks when starting fresh.
pub fn clear_taken(path_key_: &str, url: &str) {
    let mut t = taken().lock().unwrap();
    t.remove(path_key_);
    t.remove(&url_key(url));
}

/// Pause, resume or cancel one of our active downloads.
pub fn control(key: &str, action: &str) -> bool {
    let ctl = {
        let guard = jobs().lock().unwrap();
        guard.get(&path_key(key)).cloned()
    };
    let Some(ctl) = ctl else {
        return false;
    };
    match action {
        "pause" => ctl.state.store(PAUSE, Ordering::SeqCst),
        "resume" => ctl.state.store(RUN, Ordering::SeqCst),
        "cancel" => ctl.state.store(CANCEL, Ordering::SeqCst),
        _ => {}
    }
    true
}

#[derive(Clone, Debug)]
pub struct Request {
    pub url: String,
    pub path: PathBuf,
    pub cookie: Option<String>,
    pub referer: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Probe {
    pub url: String,
    pub total: u64,
    pub etag: Option<String>,
}

fn client() -> &'static reqwest::Client {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    C.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(16)
            .build()
            .expect("http client")
    })
}

fn host_of(u: &str) -> String {
    reqwest::Url::parse(u)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_lowercase()))
        .unwrap_or_default()
}

fn build_req(
    url: &str,
    req: &Request,
    origin_host: &str,
    range: &str,
    etag: Option<&str>,
) -> reqwest::RequestBuilder {
    let mut b = client()
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "*/*")
        .header("Accept-Encoding", "identity")
        .header("Range", range);
    if let Some(r) = &req.referer {
        b = b.header("Referer", r.as_str());
    }
    // Cookies belong to the origin host only (do not pass along to third-party redirects).
    if let Some(c) = req.cookie.as_ref().filter(|_| host_of(url) == origin_host) {
        b = b.header("Cookie", c.as_str());
    }
    if let Some(e) = etag {
        b = b.header("If-Range", e);
    }
    b
}

/// Ask for the first byte. HTTP 206 with Content-Range means range requests are honored.
pub async fn probe(req: &Request) -> Option<Probe> {
    let origin = host_of(&req.url);
    let resp = build_req(&req.url, req, &origin, "bytes=0-0", None)
        .send()
        .await
        .ok()?;
    if resp.status().as_u16() != 206 {
        return None;
    }
    let total = resp
        .headers()
        .get("content-range")?
        .to_str()
        .ok()?
        .rsplit('/')
        .next()?
        .trim()
        .parse::<u64>()
        .ok()?;
    let etag = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.starts_with("W/"))
        .map(|v| v.to_string());
    Some(Probe {
        url: resp.url().to_string(),
        total,
        etag,
    })
}

pub fn part_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(".odypart");
    PathBuf::from(s)
}

/// Sanitize filename: avoid Windows reserved names, invalid characters, and empty names (EDGE_CASES §5).
pub fn sanitize_filename(name: &str) -> String {
    let mut cleaned = String::new();
    for c in name.chars() {
        if "<>:\"/\\|?*".contains(c) || c.is_control() {
            cleaned.push('_');
        } else {
            cleaned.push(c);
        }
    }
    let trimmed = cleaned.trim().trim_end_matches('.').to_string();
    let stem = Path::new(&trimmed)
        .file_stem()
        .map(|s| s.to_string_lossy().to_uppercase())
        .unwrap_or_default();
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.contains(&stem.as_str()) {
        format!("_{trimmed}")
    } else if trimmed.is_empty() {
        "download".to_string()
    } else {
        trimmed
    }
}

/// Find a unique file path in `dir` avoiding collisions, e.g. `file (1).ext` (EDGE_CASES §5).
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let safe_name = sanitize_filename(name);
    let first = dir.join(&safe_name);
    if !first.exists() {
        return first;
    }
    let p = Path::new(&safe_name);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "download".into());
    let ext = p
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for i in 1..10000 {
        let candidate = dir.join(format!("{stem} ({i}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    first
}

/// Get the user's default Downloads directory.
pub fn default_download_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var_os("USERPROFILE")
            .map(|p| PathBuf::from(p).join("Downloads"))
            .unwrap_or_else(std::env::temp_dir)
    } else {
        std::env::var_os("XDG_DOWNLOAD_DIR")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join("Downloads")))
            .unwrap_or_else(std::env::temp_dir)
    }
}

fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("segdl tokio runtime")
    })
}

/// Spawn segmented download on background tokio runtime.
pub fn spawn<F: FnOnce(bool) + Send + 'static>(emit: Emit, req: Request, decide: F) {
    runtime().spawn(async move {
        run(emit, req, decide).await;
    });
}

/// Run segmented download. `decide(true)` commits to taking over; `decide(false)` falls back to engine.
pub async fn run<F: FnOnce(bool)>(emit: Emit, req: Request, decide: F) {
    let conns = connections();
    let probed = if conns > 1 {
        tokio::time::timeout(Duration::from_secs(10), probe(&req))
            .await
            .ok()
            .flatten()
    } else {
        None
    };
    let Some(p) = probed.filter(|p| p.total >= MIN_BYTES) else {
        decide(false);
        return;
    };
    let part = part_path(&req.path);
    let prealloc = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&part)
        .and_then(|f| f.set_len(p.total));
    if prealloc.is_err() {
        let _ = std::fs::remove_file(&part);
        decide(false);
        return;
    }
    let key = path_key(&req.path.to_string_lossy());
    let ctl = Arc::new(Ctl {
        state: AtomicU8::new(RUN),
    });
    jobs().lock().unwrap().insert(key.clone(), ctl.clone());
    {
        let mut t = taken().lock().unwrap();
        t.insert(key.clone());
        t.insert(url_key(&req.url));
    }
    decide(true);

    let path_str = req.path.to_string_lossy().to_string();
    let name_str = req
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "download".into());
    emit(
        "download-started",
        json!({
            "id": path_str,
            "url": req.url,
            "name": name_str,
            "path": path_str,
            "total": p.total,
        }),
    );

    let result = download(&emit, &req, &p, &part, &ctl, &key, conns).await;
    jobs().lock().unwrap().remove(&key);
    let path = req.path.to_string_lossy().to_string();
    match result {
        Ok(()) => {
            let moved = std::fs::rename(&part, &req.path).or_else(|_| {
                std::fs::copy(&part, &req.path)
                    .map(|_| ())
                    .and_then(|_| std::fs::remove_file(&part))
            });
            match moved {
                Ok(()) => emit(
                    "download-finished",
                    json!({ "id": path, "path": path, "success": true }),
                ),
                Err(e) => {
                    let _ = std::fs::remove_file(&part);
                    emit(
                        "download-finished",
                        json!({ "id": path, "path": path, "success": false, "error": e.to_string() }),
                    );
                }
            }
        }
        Err(msg) if msg == "cancelled" => {
            let _ = std::fs::remove_file(&part);
        }
        Err(msg) => {
            let _ = std::fs::remove_file(&part);
            emit(
                "download-finished",
                json!({ "id": path, "path": path, "success": false, "error": msg }),
            );
        }
    }
}

struct WorkerCtx {
    ctl: Arc<Ctl>,
    queue: Arc<Mutex<VecDeque<(u64, u64)>>>,
    done: Arc<AtomicU64>,
    alive: Arc<AtomicUsize>,
    failure: Arc<Mutex<Option<String>>>,
}

async fn download(
    emit: &Emit,
    req: &Request,
    p: &Probe,
    part: &Path,
    ctl: &Arc<Ctl>,
    key: &str,
    conns: usize,
) -> Result<(), String> {
    let total = p.total;
    let chunk = (total / (conns as u64 * 4)).clamp(1 << 20, 16 << 20);
    let mut q = VecDeque::new();
    let mut s = 0u64;
    while s < total {
        let e = (s + chunk).min(total);
        q.push_back((s, e));
        s = e;
    }
    let workers = conns.min(q.len()).max(1);
    let queue = Arc::new(Mutex::new(q));
    let done = Arc::new(AtomicU64::new(0));
    let alive = Arc::new(AtomicUsize::new(workers));
    let failure: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let origin = host_of(&req.url);

    let ctx = Arc::new(WorkerCtx {
        ctl: ctl.clone(),
        queue: queue.clone(),
        done: done.clone(),
        alive: alive.clone(),
        failure: failure.clone(),
    });

    let futs: Vec<_> = (0..workers)
        .map(|_| {
            let ctx = ctx.clone();
            worker(req, p, part, &origin, ctx)
        })
        .collect();

    let stop = Arc::new(AtomicU8::new(0));
    let (emit2, done2, alive2, stop2, ctl2) = (
        emit.clone(),
        done.clone(),
        alive.clone(),
        stop.clone(),
        ctl.clone(),
    );
    let (key2, url2) = (key.to_string(), req.url.clone());
    let reporter = async move {
        let started = Instant::now();
        let mut samples: VecDeque<(f64, u64)> = VecDeque::new();
        loop {
            let now = started.elapsed().as_secs_f64();
            let bytes = done2.load(Ordering::Relaxed);
            samples.push_back((now, bytes));
            while samples.len() > 2 && now - samples[0].0 > 3.0 {
                samples.pop_front();
            }
            let speed = match (samples.front(), samples.back()) {
                (Some(a), Some(b))
                    if b.0 - a.0 > 0.4 && ctl2.state.load(Ordering::Relaxed) == RUN =>
                {
                    (b.1.saturating_sub(a.1)) as f64 / (b.0 - a.0)
                }
                _ => 0.0,
            };
            emit2(
                "download-progress",
                json!({
                    "key": key2,
                    "uri": url2,
                    "bytes": bytes,
                    "total": total,
                    "speed": speed.round(),
                    "conns": alive2.load(Ordering::Relaxed)
                }),
            );
            if stop2.load(Ordering::Relaxed) == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
    };
    let work = async {
        join_all(futs).await;
        stop.store(1, Ordering::Relaxed);
    };
    join(work, reporter).await;

    if let Some(m) = failure.lock().unwrap().take() {
        return Err(m);
    }
    if ctl.state.load(Ordering::SeqCst) == CANCEL {
        return Err("cancelled".into());
    }
    if done.load(Ordering::Relaxed) != total {
        return Err("The download ended early".into());
    }
    Ok(())
}

async fn worker(req: &Request, p: &Probe, part: &Path, origin: &str, ctx: Arc<WorkerCtx>) {
    let mut file = match OpenOptions::new().write(true).open(part) {
        Ok(f) => f,
        Err(e) => {
            *ctx.failure.lock().unwrap() = Some(e.to_string());
            ctx.ctl.state.store(CANCEL, Ordering::SeqCst);
            ctx.alive.fetch_sub(1, Ordering::Relaxed);
            return;
        }
    };
    let mut attempts = 0u32;
    'outer: loop {
        loop {
            match ctx.ctl.state.load(Ordering::SeqCst) {
                CANCEL => break 'outer,
                PAUSE => tokio::time::sleep(Duration::from_millis(150)).await,
                _ => break,
            }
        }
        let Some((mut s, e)) = ctx.queue.lock().unwrap().pop_front() else {
            break;
        };
        while s < e {
            let range = format!("bytes={s}-{}", e - 1);
            let sent = tokio::time::timeout(
                Duration::from_secs(30),
                build_req(&p.url, req, origin, &range, p.etag.as_deref()).send(),
            )
            .await;
            let mut err: Option<String> = None;
            match sent {
                Ok(Ok(resp)) if resp.status().as_u16() == 206 => {
                    let mut stream = resp.bytes_stream();
                    if file.seek(SeekFrom::Start(s)).is_err() {
                        err = Some("Couldn't write to the disk".into());
                    }
                    while err.is_none() {
                        match ctx.ctl.state.load(Ordering::SeqCst) {
                            CANCEL => break 'outer,
                            PAUSE => break,
                            _ => {}
                        }
                        match tokio::time::timeout(Duration::from_secs(30), stream.next()).await {
                            Ok(Some(Ok(bytes))) => {
                                let take = ((e - s) as usize).min(bytes.len());
                                if file.write_all(&bytes[..take]).is_err() {
                                    err = Some("Couldn't write to the disk".into());
                                    break;
                                }
                                s += take as u64;
                                ctx.done.fetch_add(take as u64, Ordering::Relaxed);
                                attempts = 0;
                                if s >= e {
                                    break;
                                }
                            }
                            Ok(Some(Err(x))) => {
                                err = Some(x.to_string());
                            }
                            Ok(None) => {
                                if s < e {
                                    err = Some("The connection closed early".into());
                                }
                                break;
                            }
                            Err(_) => err = Some("The connection stalled".into()),
                        }
                    }
                    if ctx.ctl.state.load(Ordering::SeqCst) == PAUSE && err.is_none() {
                        break;
                    }
                }
                Ok(Ok(resp)) => {
                    let code = resp.status().as_u16();
                    if (code == 429 || code == 503 || code == 403)
                        && ctx.alive.load(Ordering::Relaxed) > 1
                    {
                        ctx.queue.lock().unwrap().push_front((s, e));
                        ctx.alive.fetch_sub(1, Ordering::Relaxed);
                        return;
                    }
                    err = Some(if code == 200 {
                        "The file changed while downloading".into()
                    } else {
                        format!("The server answered {code}")
                    });
                }
                Ok(Err(x)) => err = Some(x.to_string()),
                Err(_) => err = Some("The server took too long to answer".into()),
            }
            if let Some(m) = err {
                attempts += 1;
                if attempts > 6 {
                    *ctx.failure.lock().unwrap() = Some(m);
                    ctx.ctl.state.store(CANCEL, Ordering::SeqCst);
                    break 'outer;
                }
                tokio::time::sleep(Duration::from_millis(400 * (1 << attempts.min(4)))).await;
                continue;
            }
            if ctx.ctl.state.load(Ordering::SeqCst) == PAUSE {
                break;
            }
        }
        if s < e {
            ctx.queue.lock().unwrap().push_front((s, e));
        }
    }
    ctx.alive.fetch_sub(1, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_clamping_and_toggling() {
        set_connections(8);
        assert_eq!(connections(), 8);
        assert!(enabled());

        set_connections(1);
        assert_eq!(connections(), 1);
        assert!(!enabled());

        set_connections(32);
        assert_eq!(connections(), 16);

        set_connections(0);
        assert_eq!(connections(), 1);

        // restore default
        set_connections(8);
    }

    #[test]
    fn part_path_generation() {
        let p = Path::new("C:/Downloads/video.mp4");
        assert_eq!(
            part_path(p),
            PathBuf::from("C:/Downloads/video.mp4.odypart")
        );
    }

    #[test]
    fn path_and_url_keys() {
        assert_eq!(
            path_key("C:\\Users\\User\\Downloads\\File.PDF"),
            "c:/users/user/downloads/file.pdf"
        );
        assert_eq!(
            url_key("https://example.com/file.zip?token=123"),
            "url:https://example.com/file.zip?token=123"
        );
    }

    #[test]
    fn control_state_transitions() {
        let key = "c:/test/download.iso";
        let ctl = Arc::new(Ctl {
            state: AtomicU8::new(RUN),
        });
        jobs().lock().unwrap().insert(path_key(key), ctl.clone());

        assert!(control(key, "pause"));
        assert_eq!(ctl.state.load(Ordering::SeqCst), PAUSE);

        assert!(control(key, "resume"));
        assert_eq!(ctl.state.load(Ordering::SeqCst), RUN);

        assert!(control(key, "cancel"));
        assert_eq!(ctl.state.load(Ordering::SeqCst), CANCEL);

        jobs().lock().unwrap().remove(&path_key(key));
        assert!(!control(key, "pause"));
    }

    #[test]
    fn sanitize_and_unique_paths() {
        assert_eq!(sanitize_filename("CON.txt"), "_CON.txt");
        assert_eq!(sanitize_filename("nul"), "_nul");
        assert_eq!(
            sanitize_filename("hello:world/foo?bar.pdf"),
            "hello_world_foo_bar.pdf"
        );
        assert_eq!(sanitize_filename("   "), "download");

        let tmp = std::env::temp_dir().join(format!("segdl_unique_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let f1 = unique_path(&tmp, "test.txt");
        assert_eq!(f1, tmp.join("test.txt"));
        std::fs::write(&f1, "1").unwrap();

        let f2 = unique_path(&tmp, "test.txt");
        assert_eq!(f2, tmp.join("test (1).txt"));
        std::fs::write(&f2, "2").unwrap();

        let f3 = unique_path(&tmp, "test.txt");
        assert_eq!(f3, tmp.join("test (2).txt"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
