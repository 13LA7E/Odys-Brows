# Status

**Every agent updates this file at the end of every session**, even when stopping half way. The next agent starts here. Keep the top section short and current; the history goes in the session log.

## Now

| | |
|---|---|
| Phase | 0: Spike |
| Current task | **P0-02 CEF window on Windows** (next Windows task; P0-01b Linux pending) |
| State | **P0-01a Windows toolchains complete.** **\* LINUX PENDING \*** Linux setup and testing are deferred to the owner's Ubuntu dual boot; the full P0-01 task and Phase 0 gate remain open. |
| Next step | Read P0-02 and its edge cases, then build the Windows CEF spike. Keep P0-01b and other Linux verification explicitly pending. |
| Branch | `Constellations` on `13LA7E/Odys-Brows`. Never `main`. |
| Last pushed commit | `054e3ed` (P0-01a Windows toolchains; completion record `587eba6`) |
| Last agent | Copilot SDK in VS Code |

## Blockers

- **\* LINUX PENDING \*** P0-01b needs the deferred Ubuntu toolchain setup and version record. Windows work can continue, but the full P0-01 task and Phase 0 gate remain open.

## Questions for the owner

- Linux test setup: the owner will use an Ubuntu dual boot later; Linux setup and testing are deferred for P0-01b and P0-03.
- Open decisions Q-01 to Q-07 are listed in `docs/DECISIONS.md`. None blocks Phase 0.

## Known gaps (edge cases not handled yet)

- **P0-01b — \* LINUX PENDING \*** Linux toolchain installation, versions and CEF build packages are not verified.
- PowerShell's current execution policy blocks `npm.ps1`; use the verified `npm.cmd` shim. No execution-policy setting was changed.

## Needs testing on

- **Linux:** **\* LINUX PENDING \*** P0-01b toolchain setup and versions, plus later Linux runtime and UI verification.

## Environment (Windows recorded during P0-01a; **\* LINUX PENDING \***)

| Tool | Windows | Linux |
|---|---|---|
| OS version | Windows 11 Home Single Language 26H2, build 26300.9974, 64-bit | Pending; Ubuntu 24.04 LTS required |
| Rust (rustc --version) | rustc/cargo 1.100.0; stable-x86_64-pc-windows-msvc | Pending |
| C/C++ toolchain | VS Build Tools 2022 17.14.37906.3; MSVC 19.44.35230 x64; Windows 11 SDK 10.0.26100.0 | Pending |
| CMake / Ninja | 3.31.6-msvc6 / 1.12.1 (VS Build Tools copies added to user PATH) | Pending |
| Python | 3.14.6 | Pending |
| Node | Node 26.13.1 LTS; npm 11.20.0 via `npm.cmd` | Pending |
| CEF / cef-rs version | P0-02; not installed | P0-02; not installed |

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

### 2026-11-19: Copilot SDK in VS Code: P0-01a (Windows toolchains)
- Did: inspected the Windows toolchain; installed the official `OpenJS.NodeJS.LTS` winget package because Node/npm were missing; reused the CMake and Ninja copies already bundled with Visual Studio Build Tools and added their directories to the user PATH; recorded all Windows versions in `docs/STATUS.md`. At the owner's direction, split P0-01 into P0-01a (Windows, ticked) and P0-01b (Linux, pending) in `docs/PHASES.md`, with a prominent `* LINUX PENDING *` marker. No application code or project dependencies changed.
- Verified: `rustc --version`, `cargo --version` and `rustup show active-toolchain` reported 1.100.0 on stable-x86_64-pc-windows-msvc. `vswhere` resolved the C++ and Windows 11 SDK components to VS Build Tools 2022 17.14.37906.3; `cl` through `vcvars64.bat` reported MSVC 19.44.35230 for x64. With machine and user PATH loaded, `node --version` reported v26.13.1, `npm.cmd --version` 11.20.0, `cmake --version` 3.31.6-msvc6, `ninja --version` 1.12.1 and `python --version` 3.14.6. `winget list --id OpenJS.NodeJS.LTS --exact` confirmed the Node installation. Windows CIM/registry reported Windows 11 Home Single Language 26H2, build 26300.9974. `git diff --check` passed; `git fetch origin` and `git pull --rebase origin Constellations` reported the branch up to date; `git push origin Constellations` pushed `054e3ed` and the remote lookup matched it.
- Edge cases: §1 interrupted installation and slow/unavailable network: handled + checked by hand through winget's installer-hash verification and successful exit status; no partial install was left. Existing installations/data: handled + checked by hand by inventorying tools first and installing only missing Node.js; existing Rust, VS, Python, CMake and Ninja were preserved. Paths with spaces: handled + tested by quoting executable paths and verifying the VS-bundled CMake/Ninja PATH entries. Non-ASCII usernames and long paths: not handled (current path is ASCII; no long-path stress test applies to version checks). Both OSes: Windows handled + tested; Linux not handled because the owner deferred Linux setup. Runtime, UI, settings and web-content cases: not applicable to toolchain setup.
- Not verified / unsure: every Linux toolchain entry is unverified. PowerShell blocks `npm.ps1` under its current execution policy; `npm.cmd` is verified and no security setting was changed. No application build exists yet, so no build/test suite applies.
- Decisions proposed: none; the owner directed Windows-first work with Linux testing later.
- Next: read P0-02 and its edge cases, then build the Windows CEF spike. Resume P0-01b on the owner's Ubuntu dual boot later; the full P0-01 task and Phase 0 gate remain open.
- Commit: `054e3ed` "P0-01a: record Windows toolchains"  Pushed: yes. Completion record: `587eba6` "P0-01a: record completion"  Pushed: yes.

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
