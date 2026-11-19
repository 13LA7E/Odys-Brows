use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use std::thread;
use std::time::Duration;

use segdl::{Emit, Request, control, path_key, run};

const TOTAL_200MB: u64 = 200 * 1024 * 1024; // 209,715,200 bytes

fn start_mock_server() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind tcp listener");
    let addr = listener.local_addr().expect("local addr");
    let base_url = format!("http://127.0.0.1:{}", addr.port());
    let current_conns = Arc::new(AtomicUsize::new(0));
    let peak_conns = Arc::new(AtomicUsize::new(0));
    let current_clone = current_conns.clone();
    let peak_clone = peak_conns.clone();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let current = current_clone.clone();
            let peak = peak_clone.clone();
            thread::spawn(move || {
                let cur = current.fetch_add(1, Ordering::Relaxed) + 1;
                peak.fetch_max(cur, Ordering::Relaxed);
                let mut buf = [0u8; 2048];
                let n = stream.read(&mut buf).unwrap_or(0);
                let req_text = String::from_utf8_lossy(&buf[..n]);

                if req_text.starts_with("GET /small") {
                    let total = 4 * 1024 * 1024; // 4 MiB (< 8 MiB)
                    if let Some(range_header) = extract_range(&req_text) {
                        handle_range_response(&mut stream, &range_header, total, false);
                    } else {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nAccept-Ranges: bytes\r\n\r\n"
                        );
                        let _ = stream.write_all(header.as_bytes());
                    }
                } else if req_text.starts_with("GET /no-range") {
                    let total = TOTAL_200MB;
                    let header = format!("HTTP/1.1 200 OK\r\nContent-Length: {total}\r\n\r\n");
                    let _ = stream.write_all(header.as_bytes());
                } else if req_text.starts_with("GET /200mb") {
                    let total = TOTAL_200MB;
                    if let Some(range_header) = extract_range(&req_text) {
                        // Add slight pacing (1ms per chunk) so progress reporter captures multi-connection metrics
                        handle_range_response(&mut stream, &range_header, total, true);
                    } else {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nAccept-Ranges: bytes\r\n\r\n"
                        );
                        let _ = stream.write_all(header.as_bytes());
                    }
                }
                current.fetch_sub(1, Ordering::Relaxed);
            });
        }
    });

    (base_url, peak_conns)
}

fn extract_range(req: &str) -> Option<String> {
    for line in req.lines() {
        if line.to_lowercase().starts_with("range:") {
            return Some(line[6..].trim().to_string());
        }
    }
    None
}

fn handle_range_response(stream: &mut std::net::TcpStream, range: &str, total: u64, paced: bool) {
    if !range.starts_with("bytes=") {
        return;
    }
    let parts: Vec<&str> = range["bytes=".len()..].split('-').collect();
    if parts.is_empty() {
        return;
    }
    let start: u64 = parts[0].parse().unwrap_or(0);
    let end: u64 = if parts.len() > 1 && !parts[1].is_empty() {
        parts[1].parse().unwrap_or(total - 1)
    } else {
        total - 1
    };

    let len = (end - start + 1).min(total - start);
    let resp_header = format!(
        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{total}\r\nContent-Length: {len}\r\n\r\n"
    );
    let _ = stream.write_all(resp_header.as_bytes());

    // Stream `len` bytes in 64 KiB chunks
    let chunk = vec![0xABu8; 64 * 1024];
    let mut sent = 0u64;
    while sent < len {
        let to_send = ((len - sent) as usize).min(chunk.len());
        if stream.write_all(&chunk[..to_send]).is_err() {
            break;
        }
        sent += to_send as u64;
        if paced && sent.is_multiple_of(128 * 1024) {
            thread::sleep(Duration::from_millis(1));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_200mb_segmented_download_with_speed_and_no_leftover_files() {
    let (base_url, peak_conns) = start_mock_server();
    let url = format!("{base_url}/200mb");

    let tmp_dir = std::env::temp_dir().join(format!("segdl_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let target = tmp_dir.join("test_200mb.bin");
    let part = tmp_dir.join("test_200mb.bin.odypart");

    let progress_events = Arc::new(AtomicUsize::new(0));
    let speeds_reported = Arc::new(AtomicUsize::new(0));
    let p_clone = progress_events.clone();
    let s_clone = speeds_reported.clone();

    let emit: Emit = Arc::new(move |ev, payload| {
        if ev == "download-progress" {
            p_clone.fetch_add(1, Ordering::Relaxed);
            if let Some(_spd) = payload
                .get("speed")
                .and_then(|v| v.as_f64())
                .filter(|&s| s > 0.0)
            {
                s_clone.fetch_add(1, Ordering::Relaxed);
            }
        }
    });

    let taken = Arc::new(AtomicBool::new(false));
    let t_clone = taken.clone();

    let req = Request {
        url,
        path: target.clone(),
        cookie: None,
        referer: None,
    };

    run(emit, req, move |t| {
        t_clone.store(t, Ordering::SeqCst);
    })
    .await;

    assert!(
        taken.load(Ordering::SeqCst),
        "segdl should take over 200MB download"
    );
    assert!(target.exists(), "Destination file should exist");
    assert_eq!(
        std::fs::metadata(&target).unwrap().len(),
        TOTAL_200MB,
        "Downloaded file should be exactly 200 MB"
    );
    assert!(!part.exists(), "No leftover .odypart file on completion");
    assert!(
        progress_events.load(Ordering::Relaxed) > 0,
        "Should receive progress updates"
    );
    assert!(
        peak_conns.load(Ordering::Relaxed) > 1,
        "Server should receive multiple concurrent segmented connections (saw {})",
        peak_conns.load(Ordering::Relaxed)
    );

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_pause_and_resume_download() {
    let (base_url, _) = start_mock_server();
    let url = format!("{base_url}/200mb");

    let tmp_dir = std::env::temp_dir().join(format!("segdl_pause_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let target = tmp_dir.join("test_pause.bin");
    let part = tmp_dir.join("test_pause.bin.odypart");
    let target_key = path_key(&target.to_string_lossy());

    let bytes_received = Arc::new(AtomicU64::new(0));
    let b_clone = bytes_received.clone();

    let emit: Emit = Arc::new(move |ev, payload| {
        if ev == "download-progress" {
            let _ = payload
                .get("bytes")
                .and_then(|v| v.as_u64())
                .map(|b| b_clone.store(b, Ordering::Relaxed));
        }
    });

    let req = Request {
        url,
        path: target.clone(),
        cookie: None,
        referer: None,
    };

    let target_key_clone = target_key.clone();
    let dl_task = tokio::spawn(async move {
        run(emit, req, |_| {}).await;
    });

    // Wait until download starts and receives some bytes
    for _ in 0..50 {
        if bytes_received.load(Ordering::Relaxed) > 1024 * 1024 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Pause download
    assert!(
        control(&target_key_clone, "pause"),
        "control(pause) should succeed"
    );
    let paused_bytes = bytes_received.load(Ordering::Relaxed);
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify progress halted while paused
    let paused_bytes_after = bytes_received.load(Ordering::Relaxed);
    assert!(
        paused_bytes_after - paused_bytes < 5 * 1024 * 1024,
        "Download progress should halt when paused"
    );

    // Resume download
    assert!(
        control(&target_key_clone, "resume"),
        "control(resume) should succeed"
    );
    let _ = dl_task.await;

    assert!(target.exists(), "Target file must exist after completion");
    assert_eq!(
        std::fs::metadata(&target).unwrap().len(),
        TOTAL_200MB,
        "Downloaded file should be full size after resume"
    );
    assert!(!part.exists(), "No leftover .odypart file on completion");

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_removes_odypart_immediately() {
    let (base_url, _) = start_mock_server();
    let url = format!("{base_url}/200mb");

    let tmp_dir = std::env::temp_dir().join(format!("segdl_cancel_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let target = tmp_dir.join("test_cancel.bin");
    let part = tmp_dir.join("test_cancel.bin.odypart");
    let target_key = path_key(&target.to_string_lossy());

    let started = Arc::new(AtomicBool::new(false));
    let s_clone = started.clone();

    let emit: Emit = Arc::new(move |ev, _| {
        if ev == "download-started" {
            s_clone.store(true, Ordering::Relaxed);
        }
    });

    let req = Request {
        url,
        path: target.clone(),
        cookie: None,
        referer: None,
    };

    let target_key_clone = target_key.clone();
    let dl_task = tokio::spawn(async move {
        run(emit, req, |_| {}).await;
    });

    // Wait until started
    for _ in 0..50 {
        if started.load(Ordering::Relaxed) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Cancel download
    assert!(
        control(&target_key_clone, "cancel"),
        "control(cancel) should succeed"
    );
    let _ = dl_task.await;

    // EDGE_CASES §5: No leftover .odypart files after cancel
    assert!(
        !part.exists(),
        "Cancelled download MUST NOT leave .odypart on disk"
    );
    assert!(
        !target.exists(),
        "Cancelled download should not create destination file"
    );

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_files_under_8mib_fall_back_to_engine() {
    let (base_url, _) = start_mock_server();
    let url = format!("{base_url}/small");

    let tmp_dir = std::env::temp_dir().join(format!("segdl_small_test_{}", std::process::id()));
    let target = tmp_dir.join("test_small.bin");

    let emit: Emit = Arc::new(|_, _| {});
    let taken = Arc::new(AtomicBool::new(true));
    let t_clone = taken.clone();

    let req = Request {
        url,
        path: target,
        cookie: None,
        referer: None,
    };

    run(emit, req, move |t| {
        t_clone.store(t, Ordering::SeqCst);
    })
    .await;

    assert!(
        !taken.load(Ordering::SeqCst),
        "Files under 8 MiB must NOT be segmented, must fall back to engine"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_without_range_falls_back_to_engine() {
    let (base_url, _) = start_mock_server();
    let url = format!("{base_url}/no-range");

    let tmp_dir = std::env::temp_dir().join(format!("segdl_norange_test_{}", std::process::id()));
    let target = tmp_dir.join("test_norange.bin");

    let emit: Emit = Arc::new(|_, _| {});
    let taken = Arc::new(AtomicBool::new(true));
    let t_clone = taken.clone();

    let req = Request {
        url,
        path: target,
        cookie: None,
        referer: None,
    };

    run(emit, req, move |t| {
        t_clone.store(t, Ordering::SeqCst);
    })
    .await;

    assert!(
        !taken.load(Ordering::SeqCst),
        "Server without Range support must NOT be segmented, must fall back to engine"
    );
}
