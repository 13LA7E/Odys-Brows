# Status

**Every agent updates this file at the end of every session**, even when stopping half way. The next agent starts here. Keep the top section short and current; the history goes in the session log.

## Now

| | |
|---|---|
| Phase | 0: Spike |
| Current task | **P0-01 Toolchains** (not started) |
| State | P0-00 complete; handover and design are on `Constellations` |
| Next step | Do P0-01. Ask the owner whether Linux testing will be a VM or dual boot. |
| Branch | `Constellations` on `13LA7E/Odys-Brows`. Never `main`. |
| Last pushed commit | `9968cfb` (P0-00) |
| Last agent | Copilot SDK in VS Code |

## Blockers

- none

## Questions for the owner

- Linux test setup: VM (which software?) or dual boot? (needed for P0-01, P0-03)
- Open decisions Q-01 to Q-07 are listed in `docs/DECISIONS.md`. None blocks Phase 0.

## Known gaps (edge cases not handled yet)

- none for P0-00; runtime and UI edge cases do not apply to this Git-only task

## Needs testing on

- none for P0-00 (no application code)

## Environment (fill in during P0-01)

| Tool | Windows | Linux |
|---|---|---|
| OS version | | |
| Rust (rustc --version) | | |
| C/C++ toolchain | | |
| CMake / Ninja | | |
| Python | | |
| Node | | |
| CEF / cef-rs version | | |

---

## Session log (newest first)

Copy this template for each session:

```
### YYYY-MM-DD: <agent tool> / <model>: <task ID(s)>
- Did: <what changed, file paths>
- Verified: <commands run and their result, screenshot paths, visual_check scores>
- Edge cases: <each case for this area: handled + tested / handled + checked by hand / not handled (why)>
- Not verified / unsure: <anything you couldn't check, marked unverified>
- Decisions proposed: <D-xxx or none>
- Next: <the exact next step for the next agent>
- Commit: <hash> "<message>"  Pushed: yes/no (only finished tasks are pushed)
```

### 2026-11-19: Copilot SDK in VS Code: P0-00
- Did: initialized this checkout, recorded `origin/main` as `755c7e574bea0514effccee60893ba99b18cf820`, verified `origin/Constellations` started at `755c7e5`, checked out `Constellations`, removed Odyssey's 22 tracked files from this branch only, and added the handover, design, tools, `.gitattributes` and `.gitignore`.
- Verified: checkout had no untracked-file collisions; `git status --short` showed exactly the 22 expected Odyssey deletions plus the new project files. After pushing `9968cfb`, `git branch --show-current` printed `Constellations`, `git status --short` was empty, `git log origin/Constellations -1` showed the P0-00 commit, and `git ls-remote origin main` still returned `755c7e574bea0514effccee60893ba99b18cf820`.
- Edge cases: §1 interrupted operations and slow/unavailable network: handled + checked by hand by stopping on command failure and never rewriting pushed history. Paths with spaces: handled + tested by quoting the two Odyssey batch-file paths during `git rm`. Unexpected remote changes and untracked-file collisions: handled + tested by checking the remote tip and checkout result. Empty/missing data, long text, huge counts, repeated input, display scaling, window sizes, accessibility, settings/migrations, multiple instances, sleep/clock changes and engine/OS runtime errors: not applicable because P0-00 changes no application or UI code. Both OSes: not applicable to these Git metadata and documentation changes.
- Not verified / unsure: no application checks exist until P1-06; none apply to P0-00.
- Decisions proposed: none
- Next: P0-01; ask the owner whether Linux testing will use a VM or dual boot.
- Commit: `9968cfb` "P0-00: start Constellations with handover docs and design (Odyssey stays on main)"  Pushed: yes. This completion record is a separate same-task commit so the pushed hash is preserved.

### 2026-11-19: Claude (chat, claude.ai) / Claude Opus 5.5: handover
- Did: wrote AGENTS.md, CLAUDE.md, docs/ (STATUS, PHASES, EDGE_CASES, DECISIONS, DESIGN, HANDOVER), exported the approved design (`design/source` from the published canvas, byte-identical as checked by SHA-256), generated `design/preview` and `design/png` (2x), bundled the font, and added tools (make_previews, render_png, compare_png, visual_check).
- Verified: all 7 PNGs rendered and inspected by eye. The Private board uses its own lilac default accent. `visual_check.py --selftest` scores 0.0 on all 7 boards. `compare_png.py` calibration: identical 0, blur 0, 1 px shift 1.0, 2 px shift 8.1, different boards 22.5 or more (limit 5).
- Not verified: everything in DECISIONS marked unverified; `visual_check.py` can't run against the app until P1-04 exists.
- Known design flaw: the new-tab ring draws a "T" instead of a "+"; build the "+" (DESIGN.md §6).
- Also: checked the `Constellations` branch on GitHub: it was at `755c7e5` (same as `main`, Odyssey Phase 3). Dry-ran the P0-00 git steps (without pushing): they remove exactly Odyssey's 22 tracked files and add the handover.
- Next: P0-00.
- Commit: none (no repo yet; P0-00 creates it)
