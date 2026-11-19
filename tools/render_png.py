"""Render design/preview/*.html to design/png/*.png at 2x (2560x1600).

Needs Python Playwright and a Chrome or Chromium binary.
Usage:  python tools/render_png.py [path-to-chrome]
"""
import sys
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
PREV, PNG = ROOT / "design" / "preview", ROOT / "design" / "png"

with sync_playwright() as p:
    kw = {"args": ["--no-sandbox"]}
    if len(sys.argv) > 1:
        kw["executable_path"] = sys.argv[1]
    browser = p.chromium.launch(**kw)
    page = browser.new_page(viewport={"width": 1280, "height": 800}, device_scale_factor=2)
    PNG.mkdir(parents=True, exist_ok=True)
    for f in sorted(PREV.glob("*.html")):
        page.goto(f.resolve().as_uri())
        page.evaluate("document.fonts.ready")
        page.wait_for_timeout(300)
        page.screenshot(path=str(PNG / (f.stem + ".png")))
        print("rendered", f.stem)
    browser.close()
