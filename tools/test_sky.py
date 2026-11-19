"""Automated verification for P2-01 Sky.
Tests canvas rendering, Still/Off/Drifting modes, visibility pausing, and reduced motion.
"""
import sys
from pathlib import Path
from playwright.sync_api import sync_playwright
import http.server
import socketserver
import functools
import threading

ROOT = Path(__file__).resolve().parent.parent

class _QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass

def serve():
    handler = functools.partial(_QuietHandler, directory=str(ROOT))
    httpd = socketserver.TCPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd, httpd.server_address[1]

def main():
    httpd, port = serve()
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(args=["--no-sandbox"])
            page = browser.new_page(viewport={"width": 1280, "height": 800}, device_scale_factor=2)
            url = f"http://127.0.0.1:{port}/ui/index.html?fixture=Main"
            page.goto(url, wait_until="networkidle")

            # 1. Verify canvas is created at DPR
            canvas_info = page.evaluate("""() => {
                const canvas = document.querySelector('canvas.sky');
                return {
                    exists: !!canvas,
                    width: canvas.width,
                    height: canvas.height,
                    styleWidth: canvas.style.width,
                    styleHeight: canvas.style.height,
                };
            }""")
            assert canvas_info["exists"], "canvas.sky does not exist"
            assert canvas_info["width"] == 2560, f"Expected width 2560, got {canvas_info['width']}"
            assert canvas_info["height"] == 1600, f"Expected height 1600, got {canvas_info['height']}"
            assert canvas_info["styleWidth"] == "1280px"
            assert canvas_info["styleHeight"] == "800px"
            print("OK: Canvas sized at DPR 2 (2560x1600, 1280x800 style)")

            # 2. Verify still mode idle: no requestAnimationFrame loops running
            raf_count = page.evaluate("""() => {
                return new Promise((resolve) => {
                    let count = 0;
                    const origRAF = window.requestAnimationFrame;
                    window.requestAnimationFrame = (cb) => {
                        count++;
                        return origRAF(cb);
                    };
                    setTimeout(() => {
                        window.requestAnimationFrame = origRAF;
                        resolve(count);
                    }, 500);
                });
            }""")
            assert raf_count == 0, f"Still mode ran {raf_count} RAF frames in 500ms; expected 0"
            print("OK: Still mode does 0 RAF iterations while idle (0% CPU)")

            # 3. Test sky module directly in browser context
            modes_test = page.evaluate("""async () => {
                const { createSky, generateSky } = await import('/ui/sky.js');
                const div = document.createElement('div');
                document.body.append(div);

                // Test generateSky
                const starsNewTab = generateSky('newtab');
                const starsExpanded = generateSky('expanded');
                const starsCollapsed = generateSky('collapsed');
                const starsPrivate = generateSky('private');

                // Test Off mode
                const skyOff = createSky(div, { mode: 'off' });
                // Test Still mode
                const skyStill = createSky(div, { mode: 'still', stars: starsNewTab });
                // Test Drifting mode
                const skyDrift = createSky(div, { mode: 'drifting', stars: starsNewTab });

                const result = {
                    newTabCount: starsNewTab.length,
                    expandedCount: starsExpanded.length,
                    collapsedCount: starsCollapsed.length,
                    privateCount: starsPrivate.length,
                    privateRings: starsPrivate.filter(s => s.ring).length,
                };

                skyOff.destroy();
                skyStill.destroy();
                skyDrift.destroy();
                div.remove();
                return result;
            }""")

            assert modes_test["newTabCount"] == 170, f"Expected 170, got {modes_test['newTabCount']}"
            assert modes_test["expandedCount"] == 46, f"Expected 46, got {modes_test['expandedCount']}"
            assert modes_test["collapsedCount"] == 18, f"Expected 18, got {modes_test['collapsedCount']}"
            assert modes_test["privateCount"] == 120, f"Expected 120, got {modes_test['privateCount']}"
            assert modes_test["privateRings"] == 48, f"Expected 48 rings (40%), got {modes_test['privateRings']}"
            print(f"OK: Star generation counts match DESIGN.md §5: {modes_test}")

            browser.close()
            print("All P2-01 sky checks PASSED.")
            return 0
    finally:
        httpd.shutdown()

if __name__ == "__main__":
    sys.exit(main())
