"""Test Settings page shell, controls, accent recoloring, and persistence (P2-07)."""
import functools
import http.server
import os
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

def test_settings_interaction_and_persistence():
    httpd, port = serve()
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(args=["--no-sandbox"])
            context = browser.new_context(viewport={"width": 1280, "height": 800})
            page = context.new_page()

            url = f"http://127.0.0.1:{port}/ui/index.html?fixture=Settings"
            page.goto(url, wait_until="networkidle")
            page.wait_for_selector(".settings-content")

            # 1. Verify default initial state
            title = page.locator(".settings-content h1").text_content()
            assert title == "Appearance", f"Expected Appearance title, got {title}"

            accent_var = page.evaluate("() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim()")
            assert accent_var in ("#5ad0f0", "var(--accent-sea)"), f"Initial accent should be Sea, got {accent_var}"

            # 2. Click Mint accent and check recoloring
            page.locator("button[aria-label='Mint']").click()
            page.wait_for_timeout(100)
            accent_var = page.evaluate("() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim()")
            assert accent_var in ("#6ee7b7", "var(--accent-mint)"), f"Accent should be Mint, got {accent_var}"

            # Check that Mint is saved to localStorage
            saved = page.evaluate("() => JSON.parse(localStorage.getItem('odyssey_settings'))")
            assert saved.get("accent") == "Mint", f"localStorage should have accent Mint, got {saved}"

            # 3. Click Rose accent
            page.locator("button[aria-label='Rose']").click()
            page.wait_for_timeout(100)
            accent_var = page.evaluate("() => getComputedStyle(document.documentElement).getPropertyValue('--accent').trim()")
            assert accent_var in ("#ff8fa3", "var(--accent-rose)"), f"Accent should be Rose, got {accent_var}"

            # 4. Toggle Shooting stars switch
            switch = page.locator("button[role='switch'][aria-label='Shooting stars']")
            assert switch.get_attribute("aria-checked") == "false"
            switch.click()
            page.wait_for_timeout(100)
            assert switch.get_attribute("aria-checked") == "true"
            saved = page.evaluate("() => JSON.parse(localStorage.getItem('odyssey_settings'))")
            assert saved.get("shootingStars") is True

            # 5. Click Drifting in Stars segmented control
            page.locator(".settings-content").get_by_role("button", name="Drifting").click()
            page.wait_for_timeout(100)
            saved = page.evaluate("() => JSON.parse(localStorage.getItem('odyssey_settings'))")
            assert saved.get("stars") == "Drifting"

            # 6. Click Wide in Trail segmented control
            page.locator(".settings-content").get_by_role("button", name="Wide").click()
            page.wait_for_timeout(100)
            saved = page.evaluate("() => JSON.parse(localStorage.getItem('odyssey_settings'))")
            assert saved.get("trail") == "Wide"
            rail_var = page.evaluate("() => getComputedStyle(document.getElementById('app')).getPropertyValue('--rail').trim()")
            assert rail_var == "264px", f"Rail should be expanded (264px), got {rail_var}"

            # 7. Select 16 connections
            page.locator("button[aria-label='Faster downloads']").click()
            page.wait_for_timeout(100)
            page.locator(".settings-content").get_by_role("button", name="16 connections").click()
            page.wait_for_timeout(100)
            saved = page.evaluate("() => JSON.parse(localStorage.getItem('odyssey_settings'))")
            assert saved.get("connections") == 16

            # 8. Test Navigation to General and back
            page.locator(".settings-nav").get_by_role("button", name="General").click()
            page.wait_for_timeout(100)
            title = page.locator(".settings-content h1").text_content()
            assert title == "General", f"Expected General title, got {title}"

            page.locator(".settings-nav").get_by_role("button", name="Appearance").click()
            page.wait_for_timeout(100)
            title = page.locator(".settings-content h1").text_content()
            assert title == "Appearance", f"Expected Appearance title, got {title}"

            # 9. Test persistence across page reload (restart)
            page.reload(wait_until="networkidle")
            page.wait_for_selector(".settings-content")

            saved_after = page.evaluate("() => JSON.parse(localStorage.getItem('odyssey_settings'))")
            assert saved_after.get("accent") == "Rose", f"Expected Rose accent after reload, got {saved_after}"
            assert saved_after.get("shootingStars") is True
            assert saved_after.get("stars") == "Drifting"
            assert saved_after.get("trail") == "Wide"
            assert saved_after.get("connections") == 16
            print("All settings interaction and persistence tests passed.")

            # Clean up localStorage for other fixture tests
            page.evaluate("() => localStorage.removeItem('odyssey_settings')")
            browser.close()
    finally:
        httpd.shutdown()

if __name__ == '__main__':
    test_settings_interaction_and_persistence()
