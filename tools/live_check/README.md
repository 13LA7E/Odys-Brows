# Live checks

Drive the real debug build over Chromium's remote debugging port (no screen capture) and assert
on the UI state (`window.__uiState`) and the page targets. Each run uses an isolated profile
(`LOCALAPPDATA` override) and a local test site.

    cargo build --bin constellations
    python tools/live_check/popups_history.py   # new tabs, popup blocking, history, settings tab reuse
    python tools/live_check/ui_downloads.py      # typing survives events, sky loops, downloads
    python tools/live_check/sleep_session.py     # sleep/wake, session.json write rate
    python tools/live_check/private_ai.py        # private window, local AI against a fake server
    python tools/live_check/restore.py           # settings and tab restore (lazy)
    python tools/live_check/data_pages.py        # history ops, closed tabs, vault, internal pages
    python tools/live_check/sleep_guards.py      # audio / typed text keep tabs awake, scroll restore
    python tools/live_check/extensions.py        # unpacked extension loads after restart
    python tools/live_check/prompts_overlays.py  # permission + certificate cards, holes in page windows
    python tools/live_check/sky_idle.py          # sky stops when idle

Windows only for now. Ports 9333 and 8765 must be free. `private_ai.py` writes and deletes a
Credential Manager entry named `ai-api-key.constellations-browser`.
Not covered: real keyboard shortcuts (Ctrl+L, F6) and what is visible on screen (AUDIT-01).
