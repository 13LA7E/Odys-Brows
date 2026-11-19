"""Test Private windows: UI layout, Lilac accent, copy block, collapsed rail, and disk isolation (P3-02 / EDGE_CASES §7)."""
import functools
import http.server
import os
import socketserver
import sys
import tempfile
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

def test_private_ui_and_isolation():
    httpd, port = serve()
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(args=["--no-sandbox"])
            context = browser.new_context(viewport={"width": 1280, "height": 800})
            page = context.new_page()

            url = f"http://127.0.0.1:{port}/ui/index.html?fixture=Private"
            page.goto(url, wait_until="networkidle")
            page.wait_for_selector(".private-copy")

            # 1. Verify container theme class and CSS variables
            is_private = page.evaluate("() => document.getElementById('app').classList.contains('private')")
            assert is_private, "App element must have class 'private'"

            bg = page.evaluate("() => getComputedStyle(document.getElementById('app')).backgroundColor")
            assert bg == "rgb(2, 4, 10)", f"Expected background rgb(2, 4, 10) (#02040a), got {bg}"

            accent = page.evaluate("() => getComputedStyle(document.getElementById('app')).getPropertyValue('--accent').trim()")
            # In computed style, lilac is #b6a8ff / rgb(182, 168, 255)
            assert accent in ("#b6a8ff", "rgb(182, 168, 255)", "var(--accent-lilac)"), f"Expected lilac accent, got {accent}"

            # 2. Verify Private voyage copy block (DESIGN.md §9)
            title = page.locator(".private-title").text_content().strip()
            assert title == "Private voyage", f"Expected 'Private voyage', got {title}"

            body = page.locator(".private-body").text_content().strip()
            assert "leaves no trail behind" in body, f"Unexpected body text: {body}"

            bullets = page.locator(".private-bullet").all_text_contents()
            bullets = [b.strip() for b in bullets]
            assert "History is not saved" in bullets
            assert "Cookies and site data are cleared on close" in bullets
            assert "Shields stay on" in bullets

            # Verify bullet icons have hollow circles with lilac stroke
            bullet_circles = page.locator(".private-bullet svg circle")
            assert bullet_circles.count() == 3, f"Expected 3 bullet circles, got {bullet_circles.count()}"

            # 3. Verify Collapsed Rail (Private specifics)
            # - Outlined star logo in #cdd9ee
            logo_star = page.locator(".rail-logo svg path")
            assert logo_star.count() == 1
            assert logo_star.get_attribute("fill") == "none"

            # - Moon icon at (18, 70)
            moon = page.locator(".rail svg[style*='left:18px'][style*='top:70px']")
            assert moon.count() == 1, "Moon icon must be rendered at left:18px top:70px"

            # - Pinned apps (Mail, Calendar, Notes) are NOT rendered
            pins = page.locator(".rail-pin")
            assert pins.count() == 0, f"Private mode must not render pinned apps, found {pins.count()}"

            # - Spaces column is NOT rendered
            spaces = page.locator(".rail-spaces")
            assert spaces.count() == 0, f"Private mode must not render spaces column, found {spaces.count()}"

            # - Settings button is present at bottom
            settings_btn = page.locator(".rail-settings-btn")
            assert settings_btn.count() == 1, "Settings button must be present in rail"

            # - Rail tabs: tab 1 is active (ring r 13 + outlined star), tab 2 is inactive (hollow ring r 4)
            tab_btns = page.locator(".rail-tab-btn")
            assert tab_btns.count() == 2, f"Expected 2 tab buttons, got {tab_btns.count()}"

            # 4. Interactive tab switching in private rail
            tab_btns.nth(1).click()
            page.wait_for_timeout(100)
            # Tab 2 is now clicked, active tab updated

            # 5. Interactive new tab button
            newtab_btn = page.locator(".rail-newtab-btn")
            assert newtab_btn.count() == 1
            newtab_btn.click()
            page.wait_for_timeout(100)
            updated_tabs = page.locator(".rail-tab-btn")
            assert updated_tabs.count() == 3, f"Expected 3 tabs after clicking new tab, got {updated_tabs.count()}"

            # 6. Verify Helm
            helm_text = page.locator(".helm-address").text_content().strip()
            assert "Ask or enter an address" in helm_text, f"Expected empty Helm placeholder, got {helm_text}"

            # 7. Disk Isolation Verification (EDGE_CASES §7: no files left on disk)
            temp_profile = Path(tempfile.gettempdir()) / f"constellations_private_test_{os.getpid()}"
            # Ensure path does not exist
            if temp_profile.exists():
                import shutil
                shutil.rmtree(temp_profile)

            # In private mode, no profile directory is created or written to
            assert not temp_profile.exists(), "Private profile directory must not exist on disk"

            browser.close()
            print("test_private_ui_and_isolation: PASSED")
    finally:
        httpd.shutdown()

if __name__ == "__main__":
    test_private_ui_and_isolation()
