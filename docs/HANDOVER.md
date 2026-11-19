# Handover: background and context

Read once. It explains where this project came from, who it's for, and what to reuse.

## 1. The owner

- Syed (also goes by Sajjad). Works on Windows 11 (RTX laptop) in `C:\Projects`, is comfortable with Linux, and wants the browser on both.
- Wants plain, short updates. If you're not certain about something, say so and check it properly before stating it.
- Example sentences in UI copy, docs or tests stay generic, never about him.
- Taste: sleek, minimal, calm. He explicitly disliked UI that "looks AI-generated": glitter, glows everywhere, staggered fade-ins, generic card grids. Keep the shine subtle. He loves the stars as the product's persona.
- He hands work between AI coding agents (Claude Code, Kimi, Nemotron through OpenCode, Gemini in Antigravity). Hence `AGENTS.md` and `docs/STATUS.md`: they are the memory.
- He wants every finished part pushed to the `Constellations` branch, and edge cases considered while building (`docs/EDGE_CASES.md`).

## 2. How we got here

1. **Odyssey** (`C:\Projects\Odyssey`, private GitHub repo `13LA7E/Odys-Brows`, branch `main`) is the owner's earlier AI-first browser: Tauri 2 + WebView2, Windows only, paired with Odysseus. It went through phases 1–7 (shields, downloads, vault, themes, an aurora theme, multi-connection downloads). It works and stays as it is.
2. The owner then asked for a new design language: Arc-inspired but with its own identity, and the stars kept as the persona.
3. The first sketch looked too much like Arc and was **rejected**. Don't drift back towards Arc's look: no tab strip in a white sidebar card, no Arc-style space colour washes.
4. The second sketch introduced **the Trail** (tabs as a star trail with branches), **the Helm** (one floating bar at the bottom) and **one sky**. The owner loved it and asked for more screens; four more boards were added (Helm answer, closed tabs, private, history). That is the design in `design/`.
5. The owner decided to build it as a **separate product** on the `Constellations` branch of the same repo, for Windows and Linux, with extensions, optional AI, a proper installer, and a goal of using fewer resources than other browsers. The research behind each choice is in `DECISIONS.md`.

## 3. Odysseus (optional AI backend)

- The owner's self-hosted AI workspace on the same laptop, using Ollama, also reachable over Tailscale. Its install path, launch script and address are in Odyssey's `HANDOVER.md` §8 (`C:\Projects\Odyssey\HANDOVER.md`). Don't copy the address into this repo; read it from settings at runtime.
- Odyssey talks to it from Rust (`src/main.rs`, the "Odysseus client" part), with a token that must never reach the UI. Port that client into the `Odysseus` provider (P7-01).
- The new browser must work fully with Odysseus absent.

## 4. What to port from Odyssey

Odyssey's files (line counts from 19 Nov 2026). Read them in place; copy what you need. Don't modify Odyssey or its `main` branch.

**Where to read them:** the local checkout `C:\Projects\Odyssey` is the newest (Odyssey Phase 7, commit `1d885a4`). GitHub's `main` only has Odyssey Phase 3 (`755c7e5`) until the owner pushes the last three commits; it lacks `segdl.rs` and `fx.js` and has older `shields.rs`/`vault.rs`. If you can't reach `C:\Projects\Odyssey` and the file you need isn't on GitHub's `main`, ask the owner to push Odyssey first.

| Odyssey file | What it has | Reuse in the new browser |
|---|---|---|
| `src/segdl.rs` (407) | Multi-connection downloader: Range probe (`bytes=0-0`, needs 206 and ≥ 8 MiB), chunk queue and workers, ETag/If-Range, retries, `.odypart` then rename, pause/resume/cancel (`control`), connection setting (`set_connections`) | **Port almost as is** into `app/segdl` (P4-01). Replace the WebView2 `DownloadStarting` deferral with the CEF download handler: intercept before the native download starts, probe, then either take over or let it continue. Its `taken()`/`url_key` suppression exists only because of a wry quirk (cancelled downloads report `path: None`); keep it only if CEF has a similar problem. |
| `src/shields.rs` (841) | `adblock` crate engine (Brave's), filter-list download with a cache (`fetch_cached`, `is_fresh`), `rebuild`, cosmetic filtering (`hide_css`, `inject_css_js`, `generic_selectors`) | **Port the engine, list handling and cosmetic logic** into `app/shields` (P5-01). The WebView2 request hooks and keyboard accelerators are WebView2-specific: rewrite them against CEF's resource request handler. |
| `src/vault.rs` (291) | Password vault: JSON file, entries protected with Windows DPAPI, add/update/delete/reveal, per-origin lookup | **Port** into `app/vault` (P8-02). Keep DPAPI on Windows; on Linux use the Secret Service (for example the `keyring` crate, or libsecret directly). |
| `src/main.rs` (1394) | Windows and tabs (WebView2 child views), Odysseus client, web search, window controls, settings IPC | **Take the Odysseus client and web-search code.** The tab and window code is WebView2 and Tauri specific: don't port it, rebuild it on the engine trait. |
| `ui/app.js` (3870), `ui/app.css` (2014), `ui/index.html` | Odyssey's UI: settings, theme studio, shortcut editor, downloads list, history, shields panel | **Reuse logic, not looks.** The shortcut editor, settings persistence and the in-place download list updates are worth reading. The visual design is replaced by `DESIGN.md`. |
| `ui/fx.js` (386) | Animated sky and aurora on a canvas | Reference for P2-01 (canvas at reduced resolution, pause when hidden). The new sky is calmer: follow DESIGN.md §5. |
| `HANDOVER.md` (Odyssey) | Its architecture, gotchas and phase history (§7b–7e) | Read §4 "Gotchas" and §7e before P4-01 and P5-01. |

Lessons from Odyssey that apply here:
- **Never put entrance animations on elements that get re-rendered.** That caused the flickering downloads list and the "buttons load at different times" bug on Settings. Update rows in place, keyed by a signature (Odyssey's `dataset.sig` + `paintDownloadRows`).
- Web tabs had **no IPC**, on purpose. Keep it that way.
- Don't block requests to `127.0.0.1`, `localhost` or `*.localhost` in Shields if the shell uses local protocols.
- Test every change by building and looking at the app, not just by compiling.

## 5. Outstanding items outside this repo (owner's to do)

- Odyssey has 3 local commits not pushed to GitHub (`e5497dc`, `c583416`, `1d885a4`; local `main` is ahead of `origin/main` by 3). The owner pushes them himself (Desktop `Push Odyssey.bat`). Agents working here push only `Constellations`, never `main`, and never from `C:\Projects\Odyssey`.

## 6. The design files, and how they were made

- The design was made as a canvas of HTML boards, then exported here unchanged (`design/source/`). The published canvas link (owner-only) is in DESIGN.md §0.
- `design/generator/gen2.py` generated Main, Browsing and Settings; `gen3.py` generated the other four (it runs gen2 first). They are the exact source of every number; read the helper functions (`rail_collapsed`, `rail_expanded`, `helm`, `notch`, `star`) when building the matching component.
- `tools/make_previews.py` turns the boards into plain HTML (`design/preview/`), and `tools/render_png.py` renders those to `design/png/` at 2x. Re-run both only if `design/source/` changes, which should only happen when the owner approves a design change.
