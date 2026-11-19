"""Test Helm input classification and keyboard navigation flow (P2-06)."""
import os
import sys
import functools
import http.server
import socketserver
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

def test_helm_classification_and_interaction():
    httpd, port = serve()
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(args=["--no-sandbox"])
            page = browser.new_page(viewport={"width": 1280, "height": 800})
            
            messages = []
            page.on("console", lambda msg: messages.append(msg.text))
            url = f"http://127.0.0.1:{port}/ui/index.html?fixture=Browsing"
            page.goto(url, wait_until="networkidle")
            page.wait_for_selector(".helm")

            # 1. Test classifyInput through ES import
            results = page.evaluate("""async () => {
                const { classifyInput } = await import('./helm.js');
                return {
                    js: classifyInput('javascript:alert(1)'),
                    data: classifyInput('data:text/html,hello'),
                    chrome: classifyInput('chrome://flags'),
                    settings: classifyInput('odyssey://settings'),
                    history: classifyInput('odyssey://history'),
                    unknown: classifyInput('odyssey://foobar'),
                    winPath: classifyInput('C:\\\\Projects\\\\doc.html'),
                    fileUrl: classifyInput('file:///C:/test.html'),
                    localhost: classifyInput('localhost:3000'),
                    ip: classifyInput('127.0.0.1:8080'),
                    ipv6: classifyInput('[::1]:8080'),
                    intranet: classifyInput('router.local'),
                    domain: classifyInput('example.com/page'),
                    webUrl: classifyInput('https://google.com'),
                    search: classifyInput('search with spaces'),
                    whitespace: classifyInput('  \\n https://trimmed.com/a \\r\\n '),
                };
            }""")

            assert results['js']['kind'] == 'blocked', f"Expected blocked for js:, got {results['js']}"
            assert results['data']['kind'] == 'blocked', f"Expected blocked for data:, got {results['data']}"
            assert results['chrome']['kind'] == 'blocked', f"Expected blocked for chrome:, got {results['chrome']}"
            assert results['settings']['kind'] == 'internal' and results['settings']['page'] == 'settings'
            assert results['history']['kind'] == 'internal' and results['history']['page'] == 'history'
            assert results['unknown']['kind'] == 'internal-404'
            assert results['winPath']['kind'] == 'url' and results['winPath']['url'].startswith('file:///')
            assert results['fileUrl']['kind'] == 'url' and results['fileUrl']['url'] == 'file:///C:/test.html'
            assert results['localhost']['kind'] == 'url' and results['localhost']['url'] == 'http://localhost:3000'
            assert results['ip']['kind'] == 'url' and results['ip']['url'] == 'http://127.0.0.1:8080'
            assert results['ipv6']['kind'] == 'url' and results['ipv6']['url'] == 'http://[::1]:8080'
            assert results['intranet']['kind'] == 'url' and results['intranet']['url'] == 'http://router.local'
            assert results['domain']['kind'] == 'url' and results['domain']['url'] == 'https://example.com/page'
            assert results['webUrl']['kind'] == 'url' and results['webUrl']['url'] == 'https://google.com'
            assert results['search']['kind'] == 'search' and 'duckduckgo' in results['search']['url']
            assert results['whitespace']['kind'] == 'url' and results['whitespace']['url'] == 'https://trimmed.com/a'
            print("All input classification assertions passed.")

            # 2. Test keyboard interaction: Ctrl+L focusing Helm input
            input_el = page.locator(".helm-input")
            assert not input_el.is_visible(), "Helm input should initially be hidden"

            # Press Ctrl+L
            page.keyboard.press("Control+l")
            page.wait_for_timeout(100)
            assert input_el.is_visible(), "Helm input should be visible after Ctrl+L"

            # Press Escape
            page.keyboard.press("Escape")
            page.wait_for_timeout(100)
            assert not input_el.is_visible(), "Helm input should hide after Escape"

            # Click address box to activate
            page.locator(".helm-address").click()
            page.wait_for_timeout(100)
            assert input_el.is_visible(), "Helm input should be visible after click"

            # Type a URL and press Enter
            page.evaluate("""() => {
                window.__navigated = null;
                window.odysseyBridge = (msg) => {
                    const parsed = JSON.parse(msg);
                    if (parsed.type === 'navigate') {
                        window.__navigated = parsed.target;
                    }
                };
            }""")

            input_el.fill("https://example.org/test")
            page.keyboard.press("Enter")
            page.wait_for_timeout(100)

            nav_target = page.evaluate("() => window.__navigated")
            assert nav_target is not None, "Navigation event was not fired"
            assert nav_target['url'] == 'https://example.org/test', f"Unexpected nav target: {nav_target}"
            assert not input_el.is_visible(), "Helm input should hide after Enter"
            print("Keyboard interaction test passed.")

            browser.close()
    finally:
        httpd.shutdown()

if __name__ == '__main__':
    test_helm_classification_and_interaction()
