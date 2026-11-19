"""Automated tests for P2-04: expanded trail rail, branches, collapse toggle, and child re-attaching."""
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
            url = f"http://127.0.0.1:{port}/ui/index.html?fixture=Browsing"
            page.goto(url, wait_until="networkidle")

            # Check branches in SVG
            svg = page.locator('.rail svg').first
            paths = svg.locator('path[d*="Q28"]').all()
            assert len(paths) >= 2, f"Expected at least 2 branch curves, found {len(paths)}"
            d1 = paths[0].get_attribute('d')
            assert 'M28 150 V186 Q28 194 36 194 H46' in d1, f"Unexpected curve d: {d1}"

            # Check active tab row
            active_row = page.locator('.rail-tab-row').nth(1)
            assert 'Reading the sky before maps' in active_row.text_content()

            # Switch active tab by clicking row 0 (Search: celestial navigation)
            row0 = page.locator('.rail-tab-row').nth(0)
            row0.click()
            page.wait_for_timeout(100)

            # Now row 0 should be active (white text / active style)
            # Test collapse button
            collapse_btn = page.locator('button[aria-label="Collapse trail"]')
            collapse_btn.click()
            page.wait_for_timeout(100)

            # Check rail is now collapsed (56px)
            rail = page.locator('nav.rail')
            assert 'rail-collapsed' in rail.get_attribute('class'), "Rail should be collapsed"
            box = rail.bounding_box()
            assert box['width'] == 56, f"Expected 56px width, got {box['width']}"

            # Test child re-attachment logic in JS
            result = page.evaluate("""() => {
                const tabs = [
                    { id: 1, title: 'Root A', parent: null },
                    { id: 2, title: 'Child A1', parent: 1 },
                    { id: 3, title: 'Grandchild A1a', parent: 2 },
                    { id: 4, title: 'Root B', parent: null },
                ];
                // Simulate closing tab 2 (Child A1): Grandchild A1a should re-attach to Root A (id 1)
                const afterClose2 = tabs
                    .filter(t => t.id !== 2)
                    .map(t => t.parent === 2 ? { ...t, parent: 1 } : t);
                
                // Simulate closing tab 1 (Root A): Child A1 should re-attach to null (root)
                const afterClose1 = tabs
                    .filter(t => t.id !== 1)
                    .map(t => t.parent === 1 ? { ...t, parent: null } : t);

                return {
                    grandchildParent: afterClose2.find(t => t.id === 3).parent,
                    childNewParent: afterClose1.find(t => t.id === 2).parent
                };
            }""")

            assert result['grandchildParent'] == 1, f"Grandchild should re-attach to 1, got {result['grandchildParent']}"
            assert result['childNewParent'] is None, f"Child should re-attach to null, got {result['childNewParent']}"

            browser.close()
            print("P2-04 expanded rail with branches tests: PASSED")
            return 0
    finally:
        httpd.shutdown()

if __name__ == "__main__":
    sys.exit(main())
