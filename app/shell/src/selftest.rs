//! `--engine-selftest`: drives the real engine through the `engine-api` trait and checks the
//! events that come back (P1-02). No network needed: pages are `data:` URLs. Exit code 0 on
//! success, 1 on failure or timeout (30 s).
//!
//! Each step is checked 300 ms after loading stops, because the title event can arrive after
//! the loading event (seen when going back to a page from the back/forward cache).

use engine_api::{Engine, Event, Rect, ViewId};
use engine_cef::CefEngine;
use std::cell::RefCell;
use std::rc::Rc;

const PAGE_A: &str = "data:text/html,<title>Page A</title><p>A";
const PAGE_B: &str = "data:text/html,<title>Page B</title><p>B";
const BOUNDS: Rect = Rect {
    x: 56,
    y: 0,
    width: 1224,
    height: 800,
};

thread_local! {
    static PASSED: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Result after the engine has stopped: `Some(true)` passed, `Some(false)` failed.
pub fn result() -> Option<bool> {
    PASSED.with(|p| p.get())
}

#[derive(Default)]
struct T {
    step: u32,
    first: ViewId,
    second: ViewId,
    url: String,
    title: String,
    can_back: bool,
    log: Vec<String>,
}

impl T {
    /// The view whose events matter at this step.
    fn current(&self) -> ViewId {
        if self.step >= 5 && self.second != 0 {
            self.second
        } else {
            self.first
        }
    }
}

fn finish(ok: bool, t: &Rc<RefCell<T>>) {
    if PASSED.with(|p| p.get()).is_some() {
        return;
    }
    for line in t.borrow().log.iter() {
        println!("selftest: {line}");
    }
    println!("selftest: {}", if ok { "PASSED" } else { "FAILED" });
    PASSED.with(|p| p.set(Some(ok)));
    CefEngine.quit();
}

/// Check the current step and start the next one.
fn advance(t: &Rc<RefCell<T>>) {
    let mut eng = CefEngine;
    let (good, step) = {
        let mut s = t.borrow_mut();
        let (want, back) = match s.step {
            0 | 2 | 5 => ("Page A", false),
            _ => ("Page B", true),
        };
        let good = s.title == want && s.can_back == back;
        let line = format!(
            "step {}: title={:?} can_go_back={} url_is_data={} -> {}",
            s.step,
            s.title,
            s.can_back,
            s.url.starts_with("data:"),
            if good { "ok" } else { "WRONG" }
        );
        s.log.push(line);
        (good, s.step)
    };
    if !good {
        finish(false, t);
        return;
    }
    let (first, step_next) = {
        let mut s = t.borrow_mut();
        s.step += 1;
        (s.first, s.step)
    };
    match step {
        0 => eng.navigate(first, PAGE_B),
        1 => eng.go_back(first),
        2 => eng.go_forward(first),
        3 => eng.reload(first),
        4 => {
            // A second tab: create it, show it, hide the first (a tab switch).
            let second = eng.create_view(PAGE_A);
            {
                let mut s = t.borrow_mut();
                s.second = second;
                s.title.clear();
            }
            eng.set_bounds(second, BOUNDS);
            eng.set_visible(first, false);
            eng.set_visible(second, true);
        }
        5 => {
            let second = t.borrow().second;
            eng.close_view(second);
        }
        _ => {}
    }
    let _ = step_next;
}

pub fn start(mut engine: CefEngine) {
    let t = Rc::new(RefCell::new(T::default()));

    let sink_t = t.clone();
    engine.set_event_sink(move |e| {
        let current = sink_t.borrow().current();
        match e {
            Event::UrlChanged { view, url } if view == current => sink_t.borrow_mut().url = url,
            Event::TitleChanged { view, title } if view == current => {
                sink_t.borrow_mut().title = title
            }
            Event::LoadingChanged {
                view,
                loading: false,
                can_go_back,
                ..
            } if view == current => {
                sink_t.borrow_mut().can_back = can_go_back;
                let t2 = sink_t.clone();
                let step = sink_t.borrow().step;
                CefEngine.after(300, move || {
                    // Only if nothing else advanced the step meanwhile.
                    if t2.borrow().step == step {
                        advance(&t2);
                    }
                });
            }
            Event::Closed { view }
                if view == sink_t.borrow().second && sink_t.borrow().step == 6 =>
            {
                sink_t.borrow_mut().log.push("second view closed".into());
                finish(true, &sink_t);
            }
            Event::LoadFailed {
                url, code, text, ..
            } => {
                sink_t
                    .borrow_mut()
                    .log
                    .push(format!("load failed {code} {text} {url}"));
            }
            _ => {}
        }
    });

    let first = engine.create_view(PAGE_A);
    t.borrow_mut().first = first;
    engine.set_bounds(first, BOUNDS);
    engine.set_visible(first, true);

    let timeout_t = t.clone();
    engine.after(30_000, move || {
        timeout_t
            .borrow_mut()
            .log
            .push("timed out after 30 s".into());
        finish(false, &timeout_t);
    });
}

/// `--ipc-selftest` result reported by `ui/selftest.js` through the bridge.
pub fn ipc_result(ok: bool, log: &[String]) {
    if PASSED.with(|p| p.get()).is_some() {
        return;
    }
    for line in log {
        println!("ipc-selftest: {line}");
    }
    println!("ipc-selftest: {}", if ok { "PASSED" } else { "FAILED" });
    PASSED.with(|p| p.set(Some(ok)));
    CefEngine.quit();
}

/// Fail `--ipc-selftest` if the UI never reports (e.g. it has no bridge at all).
pub fn ipc_timeout(engine: CefEngine) {
    engine.after(30_000, || {
        ipc_result(false, &["timed out after 30 s".to_string()])
    });
}
