# Design spec

This is the design the user approved. Build it **exactly as drawn**. Every number below comes from the board source (`design/source/*.dc.html`) or the script that generated it (`design/generator/gen2.py`, `gen3.py`), not from estimates.

## 0. Source of truth, in this order

1. `design/png/*.png`: what each screen must look like, rendered at 2x (2560×1600 for a 1280×800 window). Compare your build against these side by side.
2. `design/preview/*.html`: the same boards as plain HTML that opens in any browser. Use DevTools on them to read any size, colour or position.
3. `design/source/*.dc.html` + `canvas.json`: the exact published files from the design canvas (untouched). The previews are generated from these by `tools/make_previews.py`.
4. `design/generator/gen2.py`, `gen3.py`: the Python that produced the boards. It holds the helper functions (rail, Helm, notch, star), so it is the clearest place to see how each part is built and which numbers are shared.
5. This file: the same facts organised by component, plus behaviour from the canvas notes.

The live canvas (owner-only link) is https://claude.ai/artifact/EcNLyJm6W2cqoKkcEwXp4e. The board files match it byte for byte as of 19 Nov 2026.

If the PNG and this file ever disagree, the PNG wins, and tell the owner.

To check your build against the design, use the UI's fixture mode (PHASES.md P1-04) and `python tools/visual_check.py <Board>` (see AGENTS.md "Verify").

## 1. The seven boards

| Board | File | Shows |
|---|---|---|
| New tab | `Main` | Whole window is sky; open tabs drawn as a constellation; collapsed trail; Helm empty |
| Browsing | `Browsing` | Real page, expanded trail with a branch, Helm with address and download ring, downloads popover |
| Settings | `Settings` | Internal page, Appearance section. The only interactive board (segmented controls, toggle, swatches work) |
| Helm answer | `HelmAnswer` | Question typed into the Helm, answer card above it, page dimmed |
| Closed tabs | `ClosedTabs` | Closed tabs as faint stars under the trail, hover preview with Relight and Forget |
| Private | `Private` | Private window: hollow stars, dashed trail, lilac accent |
| History | `History` | History drawn as a trail with branches, day groups, mini constellation |

## 2. The ideas (canvas notes, verbatim)

These are the user-approved descriptions written on the canvas. They define behaviour the static boards can't show.

- **The Trail.** Tabs are stars joined by a line, and tabs opened from another one branch off it. Narrow shows stars only, wide shows titles.
- **The Helm.** One command bar at the bottom, where your thumbs rest and the page keeps its full height. Downloads show as a ring around its icon.
- **One sky.** On a new tab the sky runs edge to edge and your trail is drawn as a constellation. On a real page it retreats to the rail.
- **Corner notch** for Shields and window controls. The page is square against the window and rounded only where it meets the trail.
- **Helm answers.** The bar grows into the question, and the answer sits right above it with its sources drawn as linked stars. It names where the answer came from.
- **Closed tabs** dim into faint stars below the open trail. Hover for a preview, then relight to bring the page back. They fade out after a day.
- **Private.** Stars become hollow rings, the trail line turns dashed and the accent shifts to lilac, so the mode is visible without a banner.
- **History** keeps the trail shape. Pages opened from another branch off it, and a page you returned to shows as a brighter star.

Canvas titles: "Odyssey: its own identity" (row 1) and "Next: four more screens" (row 2).

## 3. Tokens

### Colour

| Token | Value | Used for |
|---|---|---|
| `bg` | `#000000` | Window background, sky, notch, rail |
| `bg-private` | `#02040a` | Private window background |
| `surface` | `#13161a` | Helm, popovers, answer card, preview card |
| `panel-internal` | `#0a0c0f` | Internal pages (Settings, History) |
| `panel-web` | `#f8f8f6` | Stand-in for a light web page in the sketch (real pages render themselves) |
| `text` | `#e8ecef` | Main text (`MAIN`) |
| `text-strong` | `#f2f5f7` | Headings, clock, logo star, selected segment |
| `text-soft` | `#aeb7bf` | Icons, unselected tab titles (`SOFT`) |
| `text-muted` | `#7d868f` | Secondary text, placeholders (`MUTED`) |
| `pin` | `#8a949d` | Pinned-app icons in the rail |
| `node` | `#5b656f` | Inactive tab dots (`NODE`) |
| `node-faded-text` | `#6b757e` | Closed-tab titles |
| `space-dot` | `#3a424a` | Space indicator dots |
| `private-ink` | `#cdd9ee` | Private sky, private trail, private logo |
| `private-node` | `#6b7a90` | Private inactive tab rings |
| `on-accent` | `#00131b` | Icon on an accent-filled button |

Accent (user-selectable, default **Sea**): Sea `#5ad0f0`, Mint `#6ee7b7`, Lilac `#b6a8ff`, Amber `#f2c46d`, Rose `#ff8fa3`. Private windows always use Lilac `#b6a8ff`.

White overlays (all `#fff` at the given opacity): trail line `.16`, constellation line `.18`, new-tab ring stroke `.28`, active row `.07`, hover row `.05`, chip/field/segment track `.06`, selected segment `.14`, toggle off `.14`, Helm assistant button `.08`, nav selected `.08`, progress track `.10`, download ring track `.14`.

Accent at opacity: active-tab ring stroke `.4` (rail, constellation), private active ring `.45`, closed-tab hover ring `.5`.

Page dim behind a Helm answer: `rgba(0,0,0,.5)` over the page.

### Type

- One family: **Familjen Grotesk** (400, 500, 600, 700), fallback `system-ui, sans-serif`. Font files are in `design/fonts/` (SIL OFL 1.1, bundling allowed). Ship them with the app; don't load from Google at runtime.
- The serif in the Browsing board (Georgia) is the sample *website*, not the browser UI.

| Role | Size / weight / tracking / line height |
|---|---|
| Clock | 84 / 400 / -3px / 1.05 |
| Private title | 46 / 600 / -1px / 1.1 |
| Page title (Settings, History) | 32 / 600 / -0.5px |
| Wordmark "odyssey" | 17 / 600 / -0.2px |
| Private body | 16 / 400 / line 1.6 |
| Answer text | 15.5 / 400 / line 1.6 |
| Helm text, greeting, date, setting label (500), group header (600) | 15 |
| History row title | 14.5 |
| Tab rows, nav items, download name | 14 |
| Settings select | 13.5 |
| Labels, hints, descriptions, Shields count, times | 13 |
| Small meta (download detail, chips, card footer, "Closed earlier") | 12.5 |
| "Enter" hint | 12 |

History times use `font-variant-numeric: tabular-nums`.

### Shape and depth

- Radii: Helm 26 (pill), answer card 20, page panel `16 0 0 16` (left corners only), notch `0 0 0 14` (bottom-left only), popover and preview card 14, History search 12, segmented track and select 10, rows and nav items 8, chips 8, segment option 7. Round buttons are fully round.
- Shadows: Helm `0 12px 32px rgba(0,0,0,.5)`; popovers, answer card, preview card `0 16px 40px rgba(0,0,0,.55)`. Nothing else has a shadow.
- Focus: `outline: 2px solid <accent>; outline-offset: 2px` on `:focus-visible`.

### Icons

24×24 viewBox, no fill, `stroke-width 1.6`, round caps and joins, `currentColor`. The exact path data is the `I` table at the top of `gen2.py` (back, fwd, reload, dl, shield, pause, chev, mail, cal, note, folder, map, min, close, lock, side) plus `SETTINGS_ICO` (sliders), the moon in `gen3.py`, and the search icon in the History board.

### The star

Four-point star, used everywhere a "you are here" or Odysseus mark appears:

```
M12 2C12.8 8.5 15.5 11.2 22 12C15.5 12.8 12.8 15.5 12 22C11.2 15.5 8.5 12.8 2 12C8.5 11.2 11.2 8.5 12 2Z
```

Filled in normal windows. In private windows it is drawn as an outline (`stroke-width 1.6–1.8`, no fill).

## 4. Window structure

The sketch is a 1280×800 frameless window. Layers, back to front:

1. **Sky** across the whole window (§5).
2. **Trail rail** on the left: 56 wide collapsed, 264 wide expanded (§6).
3. **Page panel** from the rail's right edge to the window's right edge, full height, rounded `16 0 0 16`. Web pages and internal pages (`odyssey://settings`, `odyssey://history`) both sit here. On a new tab there is **no** panel; the sky shows through.
4. **Helm** floating over the bottom of the page area (§7).
5. **Popovers** (downloads, answer card, closed-tab preview).
6. **Notch** in the top-right corner (§8).

There is no top toolbar and no tab strip. The page keeps the full window height.

## 5. Sky

- Random white dots. Radius picked from `0.6, 0.7, 0.8, 0.9, 1.1, 1.3`; opacity from `.25, .35, .45, .55, .7`.
- New tab: 170 dots over the whole window, none within 24 px of a constellation node.
- Expanded rail behind a page: 46 dots within the 264-wide rail. Collapsed rail: 14–22 dots within the 56-wide rail.
- Private: 120 marks; 40% are hollow rings (r `1.6 / 2.0 / 2.6`, stroke `#cdd9ee` at `.25 / .35 / .5`), the rest are dots (r `.6 / .8 / 1.0`, `#cdd9ee` at `.2 / .3 / .4`).
- Settings exposes Stars: **Off / Still / Drifting** (default Still) and a **Shooting stars** toggle, "An occasional comet across the new tab" (default off). Drifting and comets are not drawn in the sketch. Keep them slow and sparse (the user asked for less glitter in Odyssey), and show the user before finalising.

## 6. Trail rail

### Collapsed (56 wide). Boards: Main, Settings, HelmAnswer, History

- Logo: star 20 `#f2f5f7` in a 32×32 button at left 12, top 14.
- Pinned apps: Mail, Calendar, Notes, 18 px icons, colour `#8a949d`, 32×32 buttons at left 12, top `68 + 36·i`.
- Tabs: centres at x 28, first at y 214, every 44 px.
  - Trunk: vertical 1 px line at x 28 from the first tab to the new-tab ring, white `.16`.
  - Active tab: circle r 13, fill `#000`, stroke accent `.4`; accent star 18 in the middle.
  - Other tabs: 7 px dot, `#5b656f`.
  - Each tab is a 32×32 round button, `aria-label "Tab N"`.
- New tab: 44 px below the last tab. Dashed circle r 10, fill `#000`, stroke white `.28`, dash `2.5 3`, with a 7 px **plus** in `#aeb7bf`, stroke 1.4.
  - **Known flaw in the sketch:** the plus is drawn as a "T" (its path is `M24.5 cy h7 M28 cy v7` shifted up 3.5, so the vertical stroke starts at the top of the bar). Build a real plus centred in the ring. Everything else about the ring is as drawn.
- Spaces: column at left 21, top 678 (window height − 122), gap 12: star 13 `#f2f5f7` (current space), then two 5 px dots `#3a424a`.
- Settings: sliders icon in a 32×32 button at left 12, top 748 (height − 52).

### Expanded (264 wide). Boards: Browsing, ClosedTabs

- Header: star 20 `#f2f5f7` centred at (22, 28); wordmark "odyssey" at left 44, top 17; collapse button (sidebar icon) at left 220, top 12.
- Pinned apps in one row at top 68, left `12 + 38·i`: Mail, Calendar, Notes, Files, Maps, and Odysseus (accent star 16).
- Rows: first centre y 150, every 44. Each row is a button at left 8, width 248, height 36, radius 8, font 14, text left at the node's x + 18.
  - Top-level tab: node at x 28. Child tab (opened from another tab): node at x 46.
  - Active row: background white `.07`, text `#e8ecef`; node is circle r 12 (fill `#000`, stroke accent `.4`) with accent star 16.
  - Other rows: text `#aeb7bf`; node is a dot r 3.5 `#5b656f`.
  - Trunk: x 28, from the first row to the last top-level row, white `.16`.
  - Branch: from the parent's node straight down x 28 to 8 px above the child, then a quadratic curve to (36, child y), then a line to x 46. Same stroke.
- New tab row 44 below the last row: the dashed ring at x 28 (plus, see above) and "New tab" in `#7d868f`.
- Bottom: at left 20, top 760: star 13 + "Home" (14, `#aeb7bf`) + two space dots. Settings button at left 220, top 748.
- Narrow vs wide is the **Trail** setting (default Narrow). The collapse button switches too.

### Closed tabs (expanded rail). Board: ClosedTabs

- Under the New tab row: "Closed earlier" (12.5, muted) at left 48.
- Closed tabs follow, 44 apart, starting 44 below New tab. Their trunk is dotted: white `.12`, dash `2 5`.
- Dots r 3.5 white, older ones dimmer: opacity `.4`, `.27`, `.16`. Titles `#6b757e`, text left 38.
- Hovered closed tab: ring r 11 (fill `#000`, stroke accent `.5`) with an accent dot r 4; row background white `.05`, title `#e8ecef`.
- Preview card beside the rail: left 276, top = row centre − 76, width 296, radius 14, `#13161a`, popover shadow.
  - Thumbnail strip 112 high (the sketch uses grey placeholder bars on `#dfe3e6`; use a real snapshot of the page).
  - Body, padding `14 16 12`: title (15/500), host (12.5 muted), "Closed 12 minutes ago" (12.5 muted, 10 above), then **Relight** (13.5/500, accent) and **Forget** (13.5, `#aeb7bf`), 18 apart.
- Closed tabs fade out after a day (canvas note).

## 7. Helm

### Frame (all boards)

- Width 560, height 52, radius 26, `#13161a`, Helm shadow. Bottom 28 from the window's bottom edge.
- Centred horizontally in the page area: `left = railWidth + (windowWidth − railWidth − 560) / 2`. That's 388 with the collapsed rail and 492 with the expanded one at 1280 wide.
- Inside: padding `0 8`, gap 2. Order: Back (32 round, icon 18), Forward (32, 18), address area (flex 1, padding `0 12`, gap 10, 15 px, single line), Reload (32, icon 17), download ring (only while downloading), 4 px spacer, Assistant (36 round, white `.08`, accent star 15).

### Address area content

- Empty: "Ask or enter an address" in muted.
- Web page: lock icon 14 muted, then host in `#e8ecef` and path in muted, as one string.
- Internal page: "odyssey://" in muted, then the page name (`settings`, `history`) in `#e8ecef`, gap 10.

### Download ring

32×32 button, `aria-label "Downloads, N percent"`. Track circle r 11 stroke 2 white `.14`; progress arc in accent, stroke 2, round cap, starting at 12 o'clock (circumference 69.1). Download arrow icon 14 in the middle. Clicking opens the downloads popover.

### Downloads popover. Board: Browsing

- Width 300, right edge aligned to the Helm's right edge, bottom 92 (sits 12 above the Helm). Radius 14, `#13161a`, popover shadow, padding `6 0`.
- Row padding `12 14`: file name (14, `#e8ecef`, ellipsis); detail line (12.5, muted, 3 below), e.g. "124 MB of 200 MB, 11.2 MB/s, 8 connections"; while active a 2 px bar 10 below (track white `.10`, fill accent).
- Right side: Pause button (icon 16) while downloading; **Open** (13, accent) when done, with the detail "Done".

### Asking (Helm answer). Board: HelmAnswer

- Same frame, padding `0 8 0 18`, gap 12. Accent star 16, the question (15, `#e8ecef`) with a caret 1.5×19 in accent, "Enter" (12, muted), then an **Ask** button: 36 round, accent fill, up-arrow 18 (stroke 2) in `#00131b`.
- The page behind is dimmed with `rgba(0,0,0,.5)`.
- Answer card: same left and width as the Helm, bottom 90, radius 20, `#13161a`, popover shadow, padding `22 24 16`.
  - Answer text 15.5 / line 1.6.
  - Sources row 18 below, gap 10: a small constellation (76×30: three white points r 3, 3.4, 2.6 joined by white `.25` lines), then one chip per source (height 26, padding `0 10`, radius 8, white `.06`, 12.5 `#aeb7bf`).
  - Footer 18 below, 12.5: who answered on the left in muted ("Answered by your Odysseus server"), **Continue in Odysseus** (13, accent) on the right.
- With the new AI providers, the footer names whichever provider answered. The sketch only shows the Odysseus case, so confirm the other wordings with the user.

## 8. Notch. All boards

- Top-right corner: height 34, `#000`, bottom-left radius 14, left padding 6.
- **Shields**: button with shield icon 16 and the blocked count ("12"), 13 px, `#aeb7bf`, padding `0 12 0 8`, gap 6, `aria-label "Shields, 12 blocked"`.
- Minimize, Maximize (rounded square), Close: 42×34 each, icons 15, `#aeb7bf`.
- On a page the notch overlaps the page panel's top-right corner. That is intended: "the page is square against the window".
- Linux: keep the same notch with our own buttons.

## 9. Screens

### New tab. Board: Main

- No page panel; collapsed rail; full sky.
- Top left (left 120, top 92): greeting "Good evening" (15, muted); clock (84, `#f2f5f7`, 4 below); date "Wednesday, 18 November" (15, muted, 2 below).
- Hint at left 120, top 262, width 260 (13, muted, line 1.5): "Your open tabs, drawn as a trail. Select a star to jump back to it."
- Constellation: each open tab is a point, joined along the trail (white `.18`, 1 px). The current tab is an accent star 20 in an accent ring r 14 (`.4`). Other points are white at `.92`, r 2.8–3.6. Labels are 13 px, 16 right of and 9 above the point's centre, `#e8ecef` for the current tab and muted for the rest. Selecting a star switches to that tab.
- The sketch's layout is illustrative: points at (520,330) (668,236) (806,352) (960,262) (888,508) (700,560), edges 0–1, 1–2, 2–3, 2–4, 4–5. The real layout must be generated from the tab tree (branches stay branches), stay clear of the clock, the hint and the Helm, and not jump around between visits.
- Helm empty.

### Browsing. Board: Browsing

- Expanded rail with the example tree: Search: celestial navigation → (Reading the sky before maps [active], Star chart (PDF)) as children; Tide tables; Lecture notes, week 9.
- Page panel from x 264. Helm shows the address and the download ring at 62%. Downloads popover open.

### Settings. Board: Settings (interactive)

- Collapsed rail (Settings tab active), page panel `#0a0c0f`, Helm shows `odyssey:// settings`.
- Left nav at left 44, top 64 within the panel, width 200, gap 2: "Settings" heading (15/600, padding `0 12 14`), then General, **Appearance** (selected: white `.08`, `#e8ecef`), New tab, Shields, Odysseus, Passwords, Privacy, Shortcuts, About. Items are 36 high, padding `0 12`, radius 8, 14 px, muted when unselected.
- Content at left 300, top 64, width 560: "Appearance" (32/600), "Make the browser feel like yours." (15, muted, 8 below), rows 28 below.
- Row: label (15/500) and description (13, muted, line 1.45, 4 below) on the left, control on the right, gap 32, padding `18 0`.
  1. **Stars**, "How the sky behaves behind the browser.": segmented Off / Still / Drifting.
  2. **Shooting stars**, "An occasional comet across the new tab.": switch.
  3. **Accent**, "Used for the active tab, progress and focus.": five swatches.
  4. **Trail**, "Show the tab trail wide with titles, or narrow with stars only.": segmented Narrow / Wide.
  5. **Faster downloads**, "Big files are fetched over several connections when the site allows it.": select, "8 connections".
- Segmented control: track padding 3, radius 10, white `.06`. Options 30 high, padding `0 14`, radius 7, 13 px. Selected: white `.14`, `#f2f5f7`. Unselected: transparent, muted.
- Switch: 42×24, radius 12. Off: white `.14`, knob left 3. On: accent, knob left 21. Knob 18 round white, top 3. `role="switch"`.
- Swatches: 22 round, gap 12. Selected: `box-shadow: 0 0 0 2px #0a0c0f, 0 0 0 4px <colour>`.
- Select: 34 high, min width 140, padding `0 12`, radius 10, white `.06`, 13.5 px, chevron 16 muted. Options 1 / 4 / 8 / 16 connections (from Odyssey).
- Defaults: Still, off, Sea, Narrow, 8 connections.
- The other sections (General, New tab, Shields, Odysseus, Passwords, Privacy, Shortcuts, About) are not drawn. Build them with exactly these row and control styles. The "Odysseus" section becomes the AI provider settings (see PHASES.md).

### Helm answer. Board: HelmAnswer

- Collapsed rail (tab 2 active), page dimmed, answer card and asking Helm as in §7.

### Closed tabs. Board: ClosedTabs

- Expanded rail with open tabs (Search, Reading [active, child], Star chart [child], Lecture notes), New tab, "Closed earlier", then Train timetable, Tide tables [hovered], Recipe: lentil soup. Preview card for Tide tables. Helm shows the address (no download ring).

### Private. Board: Private

- Background `#02040a`, private sky, accent Lilac, focus outline Lilac.
- Rail (collapsed): logo is an outlined star 20 `#cdd9ee`; a moon icon (20, accent, stroke 1.7) at left 18, top 70 replaces the pinned apps; no spaces.
  - Trunk dashed: `#cdd9ee` `.25`, dash `3 4`.
  - Active tab: circle r 13 (fill `#02040a`, stroke accent `.45`) with an outlined accent star 18.
  - Other tabs: hollow ring r 4, stroke `#6b7a90` 1.4. New-tab ring fill `#02040a`, stroke `#cdd9ee` `.3`.
- Copy at left 150, top 214, width 520: "Private voyage" (46/600, `#f2f5f7`); "This window leaves no trail behind. Nothing you open here is kept once you close it." (16, muted, 18 below); bullets 28 below, each 34 high with a hollow accent ring (r 4.5, stroke 1.3) and 15 px `#aeb7bf` text: History is not saved / Cookies and site data are cleared on close / Shields stay on.
- Helm empty.

### History. Board: History

- Collapsed rail (History tab active), panel `#0a0c0f`, Helm `odyssey:// history`.
- Inside the panel (x from the panel's left edge): "History" (32/600) at left 120, top 56. Search field at left 120, top 118, 660×46, radius 12, white `.06`, search icon 18, "Search your trail" (15, muted).
- Day groups, header 660 wide: "Today" / "Yesterday" (15/600) left, "Clear this day" (13, muted) right. Headers at top 206 and 446; the first row of each group is centred at y 262 and 502, then every 44.
- Row: left 108, width 660, height 38, radius 8. Time column 64 wide (13, muted, tabular). Trail at x 186. Title at x 216 (+20 for a branch), 14.5 px; host 10 after it (13, muted).
  - Normal page: dot r 3.4 `#5b656f`, title `#aeb7bf`.
  - Current page: ring r 11 (fill `#0a0c0f`, stroke accent `.4`) with accent star 14.
  - A page you came back to: brighter, larger white dot r 5 at `.95`.
  - Hovered row: white `.05` background, white dot, title `#e8ecef`, and on the right **Open** (accent) and **Forget** (`#aeb7bf`), 13 px, 16 apart.
  - Branch: a page opened from another one sits 20 right, joined by the same curved branch as the rail (from its parent below it, since newest is on top).
- Side panel at left 880, top 236, width 300: "Today" (15/600), summary (13, muted, line 1.5) "Four pages, one branch. The bright star is the page you came back to.", then a 300×170 mini constellation: points (40,120) (96,60) (150,104) (206,44) (254,92), edges 0–1, 1–2, 2–3, 2–4, radii 3, 3, 4.6, 3, 3, white `.9`, first point is the accent star 20 in a ring r 12.

## 10. Things the sketch does not show

Design these yourself in the same language, keep them quieter rather than louder, and **show the user a screenshot before building on them**:

- Window sizes other than 1280×800, and maximised and snapped layouts.
- Hover, pressed and disabled states (the sketch only shows hovered rows and the selected state).
- Motion. Rules carried over from Odyssey: one calm transition per user action; no staggered entrance animations; never animate children that get re-rendered (that caused flicker in Odyssey).
- Tab sleeping (PHASES.md, phase 5). Suggested, not in the sketch: a sleeping tab's star or dot is dimmer. Ask first.
- Extensions: toolbar or popup placement, the install prompt and the management page.
- AI provider setup (Ollama/LM Studio, API key, Odysseus), first-run, and the answer card for providers other than Odysseus.
- Find in page, context menus, permission prompts, error pages, print, page zoom, and the bookmarks and spaces UI beyond the dots.
- A light theme (the sketch is dark only).

## 11. Name

The boards say "odyssey", "Odyssey" and `odyssey://`. The product name for the separate browser is **not decided**, and whether the name is free to use was not checked. Keep the strings in one place so a rename is a single change.
