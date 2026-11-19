"""Automated test and screenshot capture for P2-05: page panel and layout resizing."""
import functools
import http.server
import socketserver
import sys
import threading
from pathlib import Path
from playwright.sync_api import sync_playwright

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
    shots_dir = ROOT / "docs" / "shots"
    shots_dir.mkdir(parents=True, exist_ok=True)
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(args=["--no-sandbox"])
            
            # Test widths from 900 to 2560
            test_widths = [900, 1280, 1920, 2560]
            for w in test_widths:
                page = browser.new_page(viewport={"width": w, "height": 800})
                # Test with Browsing fixture (has page panel and expanded rail)
                url = f"http://127.0.0.1:{port}/ui/index.html?fixture=Browsing"
                page.goto(url, wait_until="networkidle")
                page.evaluate("() => document.documentElement.classList.remove('fixture')")
                page.wait_for_timeout(200)

                # Query page panel and helm geometry
                geom = page.evaluate("""() => {
                    const pageEl = document.querySelector('.page');
                    const helmEl = document.querySelector('.helm');
                    const railEl = document.querySelector('.rail');
                    const pbox = pageEl ? pageEl.getBoundingClientRect() : null;
                    const hbox = helmEl ? helmEl.getBoundingClientRect() : null;
                    const rbox = railEl ? railEl.getBoundingClientRect() : null;
                    const pradius = pageEl ? window.getComputedStyle(pageEl).borderRadius : '';
                    return {
                        windowWidth: window.innerWidth,
                        windowHeight: window.innerHeight,
                        railWidth: rbox ? rbox.width : 0,
                        pageLeft: pbox ? pbox.left : 0,
                        pageWidth: pbox ? pbox.width : 0,
                        pageHeight: pbox ? pbox.height : 0,
                        pageRadius: pradius,
                        helmLeft: hbox ? hbox.left : 0,
                        helmWidth: hbox ? hbox.width : 0,
                        helmBottomOffset: hbox ? window.innerHeight - hbox.bottom : 0,
                    };
                }""")

                # Assertions
                assert geom['pageLeft'] == geom['railWidth'], f"Page should start at rail ({geom['railWidth']}), got {geom['pageLeft']}"
                assert geom['pageWidth'] == w - geom['railWidth'], f"Page width should be {w - geom['railWidth']}, got {geom['pageWidth']}"
                assert geom['pageHeight'] == 800, f"Page height should be 800, got {geom['pageHeight']}"
                assert geom['pageRadius'] == '16px 0px 0px 16px', f"Expected radius 16px 0px 0px 16px, got {geom['pageRadius']}"

                # Helm centering in page area:
                # distance from page left to helm left must equal distance from helm right to page right
                dist_left = geom['helmLeft'] - geom['pageLeft']
                dist_right = (geom['pageLeft'] + geom['pageWidth']) - (geom['helmLeft'] + geom['helmWidth'])
                assert abs(dist_left - dist_right) <= 1.0, f"Helm not centered at width {w}: left gap {dist_left}, right gap {dist_right}"
                assert abs(geom['helmBottomOffset'] - 28) <= 1.0, f"Helm bottom should be 28px, got {geom['helmBottomOffset']}"

                # Save screenshots for 1280 and 1920 per P2-05 requirements
                if w in (1280, 1920):
                    shot_path = shots_dir / f"p2-05-layout-{w}.png"
                    page.screenshot(path=str(shot_path))
                    print(f"Saved screenshot: {shot_path}")

                page.close()

            # Also verify New tab (Main fixture) has NO page panel
            page_main = browser.new_page(viewport={"width": 1280, "height": 800})
            url_main = f"http://127.0.0.1:{port}/ui/index.html?fixture=Main"
            page_main.goto(url_main, wait_until="networkidle")
            has_page = page_main.evaluate("() => document.querySelector('.page') !== null")
            assert not has_page, "New tab should have no page panel"
            page_main.close()

            browser.close()
            print("P2-05 layout and resizing tests: PASSED")
            return 0
    finally:
        httpd.shutdown()

if __name__ == "__main__":
    sys.exit(main())
