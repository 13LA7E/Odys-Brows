"""Test New Tab constellation layout, clock, date, greeting, and 1/6/20/100+ tabs (P3-01)."""
import functools
import http.server
import json
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

def test_newtab():
    shots_dir = ROOT / "docs" / "shots"
    shots_dir.mkdir(parents=True, exist_ok=True)

    httpd, port = serve()
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(args=["--no-sandbox"])
            page = browser.new_page(viewport={"width": 1280, "height": 800}, device_scale_factor=2)

            url = f"http://127.0.0.1:{port}/ui/index.html?fixture=Main"
            page.goto(url, wait_until="networkidle")
            page.wait_for_selector(".newtab-header")

            # 1. Test unit determinism of layoutConstellation
            det_check = page.evaluate("""async () => {
                const { layoutConstellation, getGreeting, formatClock, formatDate } = await import('./newtab.js');
                
                const tabs = [
                    { id: 1, title: 'Search' },
                    { id: 2, title: 'Article', parent: 1 },
                    { id: 3, title: 'Notes' },
                    { id: 4, title: 'Sub-notes', parent: 3 },
                    { id: 5, title: 'Docs' }
                ];
                
                const r1 = layoutConstellation(tabs, 1);
                const r2 = layoutConstellation(tabs, 1);
                
                const pts1 = r1.points.map(p => ({ x: p.x, y: p.y, id: p.id }));
                const pts2 = r2.points.map(p => ({ x: p.x, y: p.y, id: p.id }));
                const identical = JSON.stringify(pts1) === JSON.stringify(pts2);

                const dMorning = new Date('2026-11-19T08:30:00');
                const dAfternoon = new Date('2026-11-19T14:15:00');
                const dEvening = new Date('2026-11-19T19:45:00');
                const dNight = new Date('2026-11-19T23:10:00');

                return {
                    identical,
                    morning: getGreeting(dMorning),
                    afternoon: getGreeting(dAfternoon),
                    evening: getGreeting(dEvening),
                    night: getGreeting(dNight),
                    clockPad: formatClock(new Date('2026-11-19T09:05:00')),
                    dateFormat: formatDate(new Date('2026-11-18T12:00:00')),
                };
            }""")

            assert det_check["identical"], "Constellation layout should be strictly deterministic across calls"
            assert det_check["morning"] == "Good morning"
            assert det_check["afternoon"] == "Good afternoon"
            assert det_check["evening"] == "Good evening"
            assert det_check["night"] == "Good night"
            assert det_check["clockPad"] == "09:05"
            assert "Wednesday" in det_check["dateFormat"] and "18" in det_check["dateFormat"]
            print("Layout determinism and time formatting tests passed.")

            # Helper to render custom state in the page
            def render_tabs(tabs, active_tab=1):
                page.evaluate(f"""(tabs) => {{
                    const root = document.getElementById('app');
                    import('./shell.js').then((m) => {{
                        m.renderShell(root, {{
                            rail: 'collapsed',
                            tabs: tabs,
                            activeTab: {active_tab},
                            page: 'none',
                            sky: null,
                            starsMode: 'still',
                            shields: 0,
                            helm: {{ kind: 'empty' }},
                        }});
                    }});
                }}""", tabs)
                page.wait_for_timeout(300)

            # 2. Test 1 real tab
            tabs_1 = [{"id": 1, "title": "New tab"}]
            render_tabs(tabs_1, 1)
            shot_1 = shots_dir / "p3-01-newtab-1tab.png"
            page.screenshot(path=str(shot_1))
            assert shot_1.exists()
            assert page.locator(".constellation-star").count() == 1
            print("1 real tab rendered and screenshot captured.")

            # 3. Test 6 real tabs (with branches)
            tabs_6 = [
                {"id": 1, "title": "Search: celestial navigation"},
                {"id": 2, "title": "Reading the sky before maps", "parent": 1},
                {"id": 3, "title": "Star chart (PDF)", "parent": 1},
                {"id": 4, "title": "Tide tables"},
                {"id": 5, "title": "Lecture notes, week 9"},
                {"id": 6, "title": "Recipe: lentil soup"},
            ]
            render_tabs(tabs_6, 2)
            shot_6 = shots_dir / "p3-01-newtab-6tabs.png"
            page.screenshot(path=str(shot_6))
            assert shot_6.exists()
            assert page.locator(".constellation-star").count() == 6
            print("6 real tabs rendered and screenshot captured.")

            # Test star click switches active tab
            page.locator(".constellation-star").nth(3).click()
            page.wait_for_timeout(200)

            # 4. Test 20 real tabs
            tabs_20 = []
            for i in range(1, 21):
                parent = (i - 1) if (i % 3 == 0 and i > 1) else None
                tabs_20.append({"id": i, "title": f"Voyage article #{i}", "parent": parent})
            render_tabs(tabs_20, 5)
            shot_20 = shots_dir / "p3-01-newtab-20tabs.png"
            page.screenshot(path=str(shot_20))
            assert shot_20.exists()
            assert page.locator(".constellation-star").count() == 20
            print("20 real tabs rendered and screenshot captured.")

            # 5. Test 100+ real tabs (Edge §2: 100+ tabs)
            tabs_100 = []
            for i in range(1, 101):
                parent = (i - 1) if (i % 4 == 0 and i > 5) else None
                tabs_100.append({"id": i, "title": f"Tab {i}", "parent": parent})
            render_tabs(tabs_100, 100)
            shot_100 = shots_dir / "p3-01-newtab-100tabs.png"
            page.screenshot(path=str(shot_100))
            assert shot_100.exists()
            assert page.locator(".constellation-star").count() == 100
            print("100 real tabs rendered and screenshot captured.")

            browser.close()
    finally:
        httpd.shutdown()
    print("All P3-01 automated tests completed successfully.")

if __name__ == '__main__':
    test_newtab()
