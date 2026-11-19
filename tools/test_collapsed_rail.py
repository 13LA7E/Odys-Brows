"""Automated tests for P2-03: collapsed trail rail interactions and tab switching."""
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
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(args=["--no-sandbox"])
            page = browser.new_page(viewport={"width": 1280, "height": 800})
            url = f"http://127.0.0.1:{port}/ui/index.html?fixture=Main"
            page.goto(url, wait_until="networkidle")

            # Initial state: Tab 1 is active
            tab1 = page.locator('button[aria-label="Tab 1"]')
            tab2 = page.locator('button[aria-label="Tab 2"]')
            newtab_btn = page.locator('button[aria-label="New tab"]')

            # Tab 1 should have active SVG inside (32x32)
            assert tab1.locator('circle').count() == 1, "Tab 1 should have active circle"
            assert tab2.locator('span').count() == 1, "Tab 2 should be inactive dot"

            # Click Tab 2
            tab2.click()
            page.wait_for_timeout(100)

            # Now Tab 2 should have active circle, Tab 1 should have dot
            assert tab2.locator('circle').count() == 1, "Tab 2 should now have active circle"
            assert tab1.locator('span').count() == 1, "Tab 1 should now be dot"

            # Click New tab ring
            count_before = page.locator('.rail-tab-btn').count()
            newtab_btn.click()
            page.wait_for_timeout(100)

            count_after = page.locator('.rail-tab-btn').count()
            assert count_after == count_before + 1, f"Expected {count_before + 1} tabs, got {count_after}"

            # The new tab should be active
            newest_tab = page.locator(f'button[aria-label="Tab {count_after}"]')
            assert newest_tab.locator('circle').count() == 1, "New tab should be active"

            browser.close()
            print("P2-03 collapsed rail interaction tests: PASSED")
            return 0
    finally:
        httpd.shutdown()

if __name__ == "__main__":
    sys.exit(main())
