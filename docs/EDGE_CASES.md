# Edge cases

The owner wants edge cases handled while building, not after. For **every task**:

1. Before coding, read §1 (always) and the section(s) for your area. Add any case you can think of that isn't listed.
2. Handle each case, or decide on purpose not to (and say why).
3. Write an automated test for every case that can be tested without a person (unit or integration test). For the rest, do a manual check and write down how.
4. In the session log, list each case for your area as one of: **handled + tested**, **handled + checked by hand**, or **not handled (why)**. Copy every "not handled" into STATUS → Known gaps with the task ID.
5. If you find a new kind of case, add it to this file so the next agent sees it.

A task's "Done when" always includes: *the edge cases for its area are handled or written up as above.*

The rule of thumb: the browser must never lose the user's data, never freeze the UI, never leak private data, and never show a blank or broken screen without saying what happened and what to do.

---

## §1 Every task

- Empty, missing or null data; first run with no settings or database.
- Very long text (titles, URLs, file names): ellipsis, never overflow the design. Unicode, emoji, right-to-left text, very long words without spaces.
- Huge counts: 0, 1, 2, 100+, 1000+ items. The UI stays fast (no full re-render of long lists; update in place, see HANDOVER.md §4).
- Rapid repeated clicks or keys; an action interrupted halfway (window closed, tab closed, app quit).
- Slow, flaky or no network.
- Display scaling 100/125/150/175/200%; two monitors with different scaling; moving the window between them.
- Window sizes: the smallest allowed (set a minimum, suggest 800×600, and test it), maximised, snapped (Windows 11 snap layouts), full screen.
- Keyboard only; visible focus; screen-reader labels; reduced motion turns off drifting stars and comets.
- Both OSes. Linux: **Wayland and X11** behave differently for frameless windows, dragging, always-on-top windows and clipboard. Test both or say which you tested.
- Paths with spaces, non-ASCII user names and long paths (Windows MAX_PATH 260).
- Corrupted or half-written settings or database files: write atomically (write a temp file, then rename), recover with defaults, keep a backup, and tell the user.
- Upgrading from an older version's data (schema migrations); downgrade not supported but must not crash.
- Two copies of the app launched: single instance; a second launch opens its URL in the running app.
- Laptop sleep and resume; clock or time-zone change (anything with "today", "12 minutes ago" or 24-hour expiry).
- Errors from the engine or OS: show a calm message in the interface's voice with what to do; log details for debugging; never a silent failure.

## §2 Tabs and the Trail

- Closing the last tab (open a new tab; don't close the window unless the owner decides otherwise), closing the active tab (activate the nearest sibling, then the parent).
- Closing a parent with children: children re-attach to the parent's parent (P2-04). Deep nesting: the design shows one indent level; decide and document how grandchildren look (Owner OK).
- Opener is in another window, or the page used `noopener`/`noreferrer`: then there's no parent.
- 100+ tabs: the rail scrolls, the active tab stays in view, and the new-tab constellation stays readable (collapse or fade older stars; Owner OK).
- Tab crashed or unresponsive: a crash page with Reload; other tabs unaffected.
- Page with no title (show the host), no favicon, duplicate titles, title changing constantly (throttle UI updates).
- Pop-ups and `window.open` storms: block unrequested pop-ups, allow ones from a click.
- Tabs playing audio: indicator, mute.

## §3 Helm and addresses

- Telling a URL from a search: `localhost:3000`, `127.0.0.1`, `[::1]`, intranet names (`router.local`, `nas`), `file:///` paths, Windows paths (`C:\...`), plain words, URLs with spaces pasted.
- Pasted text with newlines or leading and trailing whitespace; very long URLs.
- Look-alike domains (IDN/punycode with mixed scripts): show punycode.
- Typed `javascript:` URLs: don't run them. `data:` top-level navigation: block. `chrome://` and `devtools://` internals: block or map to our own pages. Unknown `odyssey://` page: a clear "page not found".
- Enter pressed while a page is loading; Esc while typing; IME input (Chinese, Japanese, Korean) must not submit early.
- No network when searching.

## §4 Pages and the engine

- Certificate errors, expired certificates, HTTP-only sites, mixed content.
- HTTP basic auth prompts; `beforeunload` "leave page?" dialogs; `alert`/`confirm`/`prompt`.
- Full-screen video and the Fullscreen API: rail, Helm and notch hide, and Esc restores them.
- PDFs, print, file upload, dragging files onto the window, drag and drop between pages.
- Permission prompts: allow once, allow always, deny, and remembering per site; camera and mic in use indicator.
- External protocols (`mailto:`, app links): ask before launching another program.
- Pages that never finish loading; infinite redirects; very heavy pages.
- The Helm and popovers must never block page content the user needs: the bottom of pages should remain reachable (scrolling, cookie banners stuck to the bottom). Verify on real sites.

## §5 Downloads

- Server without Range support, or Range answered with 200: fall back to one connection.
- Unknown size; files under 8 MiB (not segmented); files over 4 GB.
- File name: already exists (`name (1).ext`), Windows-reserved names (`CON`, `NUL`, ...), invalid characters, Unicode from `Content-Disposition`, missing name.
- Disk full, no write permission, the download folder deleted.
- Network drop mid-download: retry and resume; ETag/Last-Modified changed: restart cleanly, never mix parts.
- Pause, then quit the app: say whether it resumes after restart (Odyssey's didn't).
- Cancel during the Range probe; the same file downloaded twice at once.
- Downloads that need the page's cookies or auth headers, signed URLs that expire, redirects: the segmented downloader must send the same credentials as the engine would, or fall back to the engine's download.
- Private-window downloads: the file is kept, nothing is written to history.
- No leftover `.odypart` files after cancel, failure or crash cleanup.

## §6 History, closed tabs and sessions

- Private windows never write history, closed tabs or session data.
- Day grouping across midnight and time-zone changes; "Clear this day" on today while browsing.
- The same URL visited many times ("came back" brighter star, no duplicates flooding the list).
- Closed tab exactly at 24 h; relighting a page that now fails; never resubmit a POST form when relighting or restoring.
- Session restore after a crash with 100+ tabs: restore lazily (sleeping) so start-up stays fast.
- Search with special characters (`%`, `_`, quotes) in SQL: use parameters, never string-built SQL.

## §7 Private windows

- Separate in-memory profile: no cookies, cache or storage written to disk; verify by checking the profile folder after closing.
- Several private windows at once: decide whether they share one session (document it).
- Extensions off in private windows unless the user allows each one.
- AI in a private window: never store the conversation; make it clear what is sent to the provider.
- Crash while private: nothing restored.

## §8 Shields

- Filter lists fail to download: use the bundled or last cached lists, and never block browsing.
- Sites that break: a per-site off switch that works immediately.
- Never block our own local protocols, `localhost` or `127.0.0.1` (HANDOVER.md §4).
- The counter in the notch resets per page and never shows nonsense (negative, huge).

## §9 Tab sleeping and efficiency

- Never sleep: the active tab, tabs playing audio or in a call, tabs with unsaved form input, tabs with active downloads, pinned tabs (default), tabs with devtools open.
- Waking restores the URL and scroll position; pages that can't be restored safely (POST) show a "reload?" state instead of resubmitting.
- Low-memory pressure from the OS: sleep more aggressively, but never lose a tab.

## §10 Extensions

- Installing an MV2 extension: a clear message that it's not supported, not a crash.
- An extension crashing or hanging; one asking for broad permissions (show them before install).
- Extension popups must appear above pages (the same z-order issue as the Helm).
- Extension updates failing; removing an extension clears its data.
- Extensions that open tabs or windows (`chrome.tabs.create`, an install "welcome" page): they must land in our Trail, never in a native Chrome window (found in P0-05).
- Extensions other programs register for Google Chrome (Windows registry, Linux external extension folders): never pick them up or download them (found in P0-05).
- An extension page as the very first URL at start-up fails until extensions have registered (`ERR_BLOCKED_BY_CLIENT`, found in P0-05): restore such tabs after extensions are ready.

## §11 AI

- No provider set: the Assistant button explains how to set one up. The browser is fully usable.
- Provider offline, timeouts, wrong key (401), rate limited (429), model missing, server error: clear message with what to do; never a spinner forever.
- Streaming cut off midway; the user cancels (Esc); a new question while one is streaming.
- Very large pages: truncate to the model's context in a sensible way and say so.
- Non-English pages and questions.
- Never send passwords, cookies, form contents or private-window data unless the user explicitly asks; never put keys or the Odysseus token in the UI or logs.
- Page text is untrusted: content on a page must never trigger browser actions through the AI (prompt injection).

## §12 Window, notch and system

- Frameless window: dragging, double-click to maximise, snap layouts on Windows 11, resize edges; on Linux Wayland, moving the window must go through the compositor (no manual positioning).
- Restoring the window position when that monitor is gone.
- Default-browser registration and opening links from other apps (later phase, but don't block it in the design).

## §13 Install and update

- Installing without admin rights; upgrading keeps the profile; uninstall asks whether to remove the profile (ask the owner what the default should be).
- An update interrupted halfway; no network when checking for updates; a failed update must leave the old version working.
- AppImage on a system without FUSE: document the workaround.
