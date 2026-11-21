"""Screenshot our browser UI in fixture mode and compare it with the approved design.

Usage:
  python tools/visual_check.py [BOARD ...] [--region x,y,w,h] [--threshold 5.0]
                               [--report-only] [--selftest] [--chrome PATH]

  BOARD        one or more of: Main Browsing Settings HelmAnswer ClosedTabs Private History
               (default: all)
  --region     compare only this part, in 1280x800 design coordinates
  --report-only  always exit 0 (used by tools/check.* while screens are unfinished)
  --selftest   screenshot design/preview instead of our UI; every board must score ~0.
               Use it to check that this tool and your Playwright setup work.
  --chrome     path to a Chrome/Chromium binary if Playwright's own isn't installed

How it works: serves the repository root on a local port, opens
  /ui/index.html?fixture=<Board>
at 1280x800 with device scale 2 (same as design/png), waits for fonts, saves
out/visual/<Board>.png and out/visual/<Board>.diff.png, and prints a score.

The fixture contract (built in PHASES.md P1-04): with ?fixture=<Board> the UI renders
that board's exact sample content with no engine. The sky must use the same star
positions as the board (read them from design/preview/<Board>.html) so the score is fair.

Needs: pip install playwright pillow ; python -m playwright install chromium
"""
import argparse
import functools
import http.server
import socketserver
import sys
import threading
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from compare_png import compare  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
BOARDS = ["Main", "Browsing", "Settings", "HelmAnswer", "ClosedTabs", "Private", "History"]


class _QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass


def serve():
    handler = functools.partial(_QuietHandler, directory=str(ROOT))
    # Threaded, with a long queue: the UI is many small modules the browser asks for at once, and a
    # one-request-at-a-time server drops some of them (the first board then fails to load).
    class _Server(socketserver.ThreadingMixIn, socketserver.TCPServer):
        daemon_threads = True
        request_queue_size = 128
        allow_reuse_address = True

    httpd = _Server(("127.0.0.1", 0), handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd, httpd.server_address[1]


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("boards", nargs="*", default=BOARDS)
    ap.add_argument("--region")
    ap.add_argument("--threshold", type=float, default=5.0,
                    help="pass limit for the score (see tools/compare_png.py), default 5")
    ap.add_argument("--report-only", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--chrome")
    a = ap.parse_args(argv)

    bad = [b for b in a.boards if b not in BOARDS]
    if bad:
        print("unknown board(s): %s. Choose from: %s" % (", ".join(bad), " ".join(BOARDS)))
        return 2
    if not a.selftest and not (ROOT / "ui" / "index.html").exists():
        print("No ui/index.html yet: fixture mode arrives in PHASES.md P1-04. "
              "Run with --selftest to check the tool itself.")
        return 0 if a.report_only else 2

    try:
        from playwright.sync_api import sync_playwright
    except ImportError:
        print("Playwright is missing: pip install playwright pillow && python -m playwright install chromium")
        return 0 if a.report_only else 2

    out = ROOT / "out" / "visual"
    out.mkdir(parents=True, exist_ok=True)
    httpd, port = serve()
    failures = 0
    try:
        with sync_playwright() as p:
            kw = {"args": ["--no-sandbox"]}
            if a.chrome:
                kw["executable_path"] = a.chrome
            browser = p.chromium.launch(**kw)
            page = browser.new_page(viewport={"width": 1280, "height": 800}, device_scale_factor=2)
            errors = []
            page.on("pageerror", lambda e: errors.append(str(e)))
            for b in a.boards:
                errors.clear()
                url = ("http://127.0.0.1:%d/design/preview/%s.html" % (port, b) if a.selftest
                       else "http://127.0.0.1:%d/ui/index.html?fixture=%s" % (port, b))
                page.goto(url, wait_until="networkidle")
                page.evaluate("document.fonts.ready")
                page.wait_for_timeout(400)
                shot = out / ("%s.png" % b)
                page.screenshot(path=str(shot))
                score, raw = compare(ROOT / "design" / "png" / ("%s.png" % b), shot,
                                     a.region, out / ("%s.diff.png" % b))
                ok = score <= a.threshold and not errors
                failures += 0 if ok else 1
                print("%-11s %s  score %.1f (limit %.1f)  %.2f%% of pixels%s"
                      % (b, "PASS" if ok else "FAIL", score, a.threshold, raw,
                         ("  JS errors: " + " | ".join(errors[:3])) if errors else ""))
            browser.close()
    finally:
        httpd.shutdown()
    print("Screenshots and diffs: %s" % out.relative_to(ROOT))
    return 0 if (failures == 0 or a.report_only) else 1


if __name__ == "__main__":
    sys.exit(main())
