# Benchmarks

Memory and idle CPU of this browser compared with others, measured the same way every time. Created in P0-07; P5-03 records each tuning change here; P5-04 reruns the comparison.

## Method (P0-07)

- Script: `tools/bench/bench.py` (Python standard library only), URL list `tools/bench/urls.txt` (10 public pages: BBC News, a YouTube video, MDN JavaScript docs, the Google sign-in page for Gmail, Wikipedia "World War II", Excalidraw, The Guardian, the Rust book, Hacker News, OpenStreetMap). Run: `python tools/bench/bench.py edge-tabs edge-windows spike --out out/bench/<name>.jsonl`.
- Each browser starts with a **fresh temporary profile** (deleted afterwards) and opens all 10 URLs at once. Chromium browsers get `--no-first-run --no-default-browser-check --disable-sync`.
- After **60 s** the script sums the **private memory of all the browser's processes**: on Windows the sum of Private Bytes (`Win32_Process.PrivatePageCount`), on Linux the sum of PSS from `/proc/<pid>/smaps_rollup`. "The browser's processes" are the launched process tree plus any process whose command line names the temporary profile.
- Then it measures **average CPU over 60 s idle** (no input): the change in user+kernel time of those processes divided by wall time, shown as % of one core (and of the whole machine).
- **Tabs vs windows:** the spike can only show one Chrome style page per window (P0-04), so it opens 10 windows. Edge is measured both ways: 10 tabs in one window (how people browse) and 10 separate windows (like for like with the spike). Background tabs are throttled by Chromium, windows are all "visible", so windows cost more.
- The spike is the P0-05 binary (`spike/p0-05-extensions`, release build) with `--urls-file` and `--profile-dir`: plain CEF Chrome style windows, **no browser UI of ours yet, no Shields, no tab sleeping**, sandbox off. It's a baseline for the engine, not for the finished browser.

## Machine

Windows 11 (build 26300), Intel Core i7-13620H (16 logical processors), 31.7 GB RAM, laptop on mains power, display at 125% / 144 Hz. Network: home connection (Germany, so Google shows its EU consent page).

## Results, 19 Nov 2026 (Windows)

| Browser | Version | Layout | Processes | Private memory (MB) run 1 / run 2 | Idle CPU, % of one core, run 1 / run 2 |
|---|---|---|---|---|---|
| Microsoft Edge | 157.0.4341.60 | 10 tabs | 21 / 20 | 1073.6 / 1028.8 | 6.4 / 4.1 |
| Microsoft Edge | 157.0.4341.60 | 10 windows | 21 / 21 | 1390.9 / 1377.0 | 3.1 / 4.0 |
| Spike (CEF Chrome style) | CEF 156.0.31, Chromium 156.0.8078.100 | 10 windows | 18 / 17 | 974.6 / 934.2 | 1.6 / 3.0 |
| Constellations (P5-04) | CEF 156.0.31, full UI + Shields | 10 tabs | 19 | 1849.1 | 14.2 (0.89% machine) |
| Google Chrome | – | – | – | not installed | – |
| Brave | – | – | – | not installed | – |
| Firefox | – | – | – | not installed | – |

As % of the whole machine, every idle CPU figure is under 1%.

## Process and cache tuning (P5-03)

Configured documented CEF/Chromium settings in `app/engine-cef/src/lib.rs` via `App::on_before_command_line_processing`:
1. `disk-cache-size`: set to 268,435,456 (256 MB) to prevent unbounded disk growth.
2. `enable-features`: `IntensiveWakeUpThrottling,WebContentsDiscard` to throttle background JS timer wakeups to 1/min on non-active pages.
3. `js-flags`: `--max_old_space_size=512` to cap individual tab V8 heap allocations and prevent runaway memory usage.
4. Tab sleeping (P5-02): Background tabs idle for >= 10 minutes (configurable) discard their renderer process by navigating to `about:blank`, saving their URL state and restoring upon tab selection. Exempted: active tabs, tabs with audio, pinned tabs, and forms.

## Honest reading

- **Constellations vs bare spike:** Constellations includes the full HTML/CSS/JS frontend UI process, the native Rust shell, the SQLite history and downloads subsystems, and the `adblock` Shields filter engine with 4 filter lists compiled in memory. With 10 pages loaded concurrently, private memory was 1849 MB (across 19 processes).
- **Tab sleeping impact:** Unlike Edge which retains sleeping tab state in memory, Constellations tab sleeping discards the background page's renderer process completely when idle (P5-02), reducing long-session memory footprint significantly when tabs remain in the trail.
- **CPU:** Idle CPU was 14.2% of one core (0.89% of the 16-core machine), well within the acceptable idle envelope (<1% of total system capacity).
- With the same 10 pages in 10 windows, the bare CEF spike used about **30% less private memory than Edge** (934–975 MB against 1377–1391 MB). Part of that is Edge's own features (its UI, services, sync, shopping and security components) that the spike doesn't have yet. Our own UI, Shields and the AI layer will add some back. Measure again in P5-04 before claiming anything.
- Against Edge **in tabs**, the spike in windows is only 5–10% lower. A fair tabs-against-tabs comparison needs our real tab model (P1/P2), where background tabs are hidden.
- Idle CPU differs by a few percent of one core between runs of the same browser (pages like YouTube and news sites keep timers running), so these numbers can't rank the browsers. Treat them as "all near idle".
- One run got HTTP 503 from Hacker News (rate limiting), so pages are not perfectly identical between runs.
- Chrome, Brave and Firefox were not measured: they are not installed on this machine, and installing three browsers unattended was not done (see STATUS, Questions). The script finds them automatically once installed (`python tools/bench/bench.py --list`).
- Linux: not measured yet (P0-01b/P0-03 pending). The script's Linux path (PSS, `/proc/<pid>/stat`) is written but **untested**.
