# AGENTS.md

Instructions for any coding agent working in this repository: Claude Code, Kimi Code, OpenCode (for example with Nemotron), Google Antigravity (for example with Gemini 3.8 Flash), Codex or anything else. The owner hands work between agents, so **the next agent knows only what is written in this repository.** Write everything down.

## What this project is

A new desktop web browser for **Windows and Linux** (no macOS). It has its own visual identity: tabs drawn as a trail of stars, one floating command bar at the bottom (the Helm), and a sky behind everything. It has optional AI that works with a local model, an API key or the owner's self-hosted Odysseus server, and the browser must work fully with no AI set up at all. It is a separate product from the owner's earlier browser, Odyssey (`C:\Projects\Odyssey`), which we copy useful code from.

The product name is not decided yet. The design says "odyssey" for now. The working name of the project and its branch is **Constellations**.

## Where the code lives

- GitHub: `https://github.com/13LA7E/Odys-Brows`, branch **`Constellations`**. All work for this project goes on that branch.
- `main` in the same repo is the old Odyssey browser. **Never commit to, push to, merge into or rebase `main`.**
- Local folder on the owner's PC: `C:\Projects\NewBrowser` (becomes a git checkout of `Constellations` in task P0-00).

## Read these, in this order, at the start of every session

1. `docs/STATUS.md`: where the work is, what the last agent did, and **the next task**. Always read this first.
2. `docs/PHASES.md`: the task list. Find the task named in STATUS and read only that task and its phase header.
3. `docs/EDGE_CASES.md`: §1 plus the section for your task's area. Every task must handle these.
4. `docs/DECISIONS.md`: decisions already made. Don't reopen an Accepted decision without the owner's approval.
5. `docs/DESIGN.md` + `design/png/`: the approved design. Read this before any UI work.
6. `docs/HANDOVER.md`: background, the owner, the old Odyssey code and what to port from it. Read it once, and again when a task points at it.

If your context window is small, read STATUS, then the one task, then only the sections that task links to.

## Session protocol (everyone follows this)

**Start**
1. Run `git status`, `git branch --show-current` (must say `Constellations`), `git fetch origin` and `git log --oneline -10 origin/Constellations`. If the remote branch has commits you don't have, `git pull --rebase origin Constellations` first. If there are uncommitted changes you didn't make, stop and ask the owner. (Before P0-00 there is no git repo yet: go straight to P0-00.)
2. Read `docs/STATUS.md` and the task it names. Work on **that task only**. If it's unclear or blocked, write the question in STATUS under "Questions for the owner" and ask.

**While working**
- Keep the change small enough to finish and verify in one session. If a task is too big, split it into sub-tasks in PHASES.md (`P2-03a`, `P2-03b`) before starting.
- **Edge cases as you build**, not afterwards: work through `docs/EDGE_CASES.md` §1 and your area's section, write tests for the ones that can be automated, and record each one in the session log as handled + tested / handled + checked by hand / not handled (why). Add new kinds of cases to that file.
- Run the checks in "Verify" before calling anything done.
- Never mark a task done unless every line of its "Done when" is true, including the edge cases. Paste the evidence (command output, screenshot path, numbers) into the session log.

**End (mandatory, even if you stop half way)**
1. Update `docs/STATUS.md`: current task and its state, what changed, what's next, blockers, known gaps and questions for the owner.
2. Add an entry at the **top** of the session log in STATUS (template is in that file).
3. Tick finished tasks in `docs/PHASES.md`.
4. If you made a decision that future work depends on, add it to `docs/DECISIONS.md` as Proposed and ask the owner.
5. Commit with the task ID: `P2-03: collapsed trail rail`. One task per commit where possible. Stopping mid-task: commit as `WIP P2-03: ...` and don't push it.
6. **When a task is done, push it** (see "Git and pushing"). The owner wants every finished part on GitHub.

## Git and pushing

- Branch: `Constellations` only. Remote: `origin` = `https://github.com/13LA7E/Odys-Brows.git`.
- Push after **every finished task** (all "Done when" lines true, the check passes, STATUS and PHASES updated in the same commit or the one before): `git push origin Constellations`.
- Before pushing: `git fetch origin` and `git pull --rebase origin Constellations`. If the rebase has conflicts you can't resolve with certainty, stop and ask the owner. After a rebase, run the check again.
- **Never** force-push, never rewrite commits that are already pushed, never push WIP commits, never push to `main`, never commit secrets (API keys, tokens, certificates, `.env`).
- If the push fails because of authentication, stop and tell the owner. Don't create tokens, change credentials or switch the remote URL.
- After pushing, write the pushed commit hash in the session log.

## Hard rules

- **Design is fixed.** Build what `design/png/*.png` shows, using the numbers in `docs/DESIGN.md`. Don't restyle, "improve", add gradients, glows, glitter or extra animation. If something isn't designed (`DESIGN.md` §10), design it in the same language, keep it quiet, and get the owner's OK with a screenshot.
- **Windows and Linux both.** Every feature must build and run on both. If you can only test one, say so in the session log and add a line to STATUS "Needs testing on: Linux" (or Windows).
- **Web content never gets IPC.** Only our own UI talks to the native side. Never pass secrets (API keys, the Odysseus token, passwords) to the UI or to web pages.
- **Don't guess facts.** If you're not sure about an API, version or behaviour, look it up in the official docs or the source, and say in the log what you checked. If you can't check, write "unverified" next to it. The owner wants this.
- **Don't delete or rewrite** `design/source/`, `design/png/` or `docs/DECISIONS.md` history. Mark a decision Superseded; don't erase it.
- **No new dependencies** without a one-line reason in the commit message. Prefer well-maintained crates and packages.
- **Keep the build warning-free** (`cargo clippy -- -D warnings` once the code exists).
- **Don't run your tool's `/init`.** Kimi Code, OpenCode and Claude Code all have an init command that writes or overwrites `AGENTS.md`/`CLAUDE.md`. This file is maintained by hand.
- **Don't add `GEMINI.md`, `.agents/rules/` or other tool-specific rule files.** Antigravity gives `GEMINI.md` priority over `AGENTS.md`, so a second file would quietly override this one. Every rule goes in this file.
- Ask the owner before anything that costs money, signs or publishes a release, changes accounts, or can't be undone. (Pushing finished tasks to `Constellations` is already approved.)

## Repository layout

```
AGENTS.md            this file (CLAUDE.md just imports it)
docs/
  STATUS.md          live state + session log  <- start here
  PHASES.md          phased task list with "Done when" checks
  EDGE_CASES.md      edge cases every task must handle, by area
  DECISIONS.md       decision records (D-001 ...)
  DESIGN.md          the design spec with exact numbers
  HANDOVER.md        background, owner, Odyssey port map
  BENCHMARKS.md      created in P0-07: memory/CPU numbers
design/
  png/               approved screens at 2x (2560x1600)  <- the target
  preview/           same screens as plain HTML (open in a browser, inspect with DevTools)
  source/            exact files from the design canvas (do not edit)
  generator/         Python that generated the boards (exact helper code)
  fonts/             Familjen Grotesk woff2 + OFL licence
tools/
  make_previews.py   design/source -> design/preview
  render_png.py      design/preview -> design/png
  compare_png.py     diff two PNGs, prints a score, writes a diff image
  visual_check.py    screenshots our UI in fixture mode and compares with design/png
spike/               Phase 0 experiments (throwaway)
app/, ui/            created in Phase 1 (see PHASES.md P1-01)
```

## Verify

Until Phase 1 exists there is no app to build. After P1-06 the single command is:

- Windows: `powershell -ExecutionPolicy Bypass -File tools\check.ps1`
- Linux: `bash tools/check.sh`

Visual check (needs `pip install playwright pillow` and `python -m playwright install chromium`):

```
python tools/visual_check.py            # all boards that have a fixture
python tools/visual_check.py Main       # one board
python tools/compare_png.py design/png/Main.png out/Main.png   # any two images
```

`python tools/visual_check.py --selftest` checks the tool itself (every board must score 0). The score is the share of drawn pixels that differ; 5 or less passes (a 2 px misplacement already fails). If your tool can show you images, also look at `out/visual/<Board>.diff.png` and the screenshot itself. A low score is necessary but not enough: check it with your eyes, or ask the owner to.

## Environment

- Owner's main machine: Windows 11 laptop with an NVIDIA RTX GPU. Shell: PowerShell. Projects live in `C:\Projects`.
- Linux testing: a VM or dual boot (to be set up in P0-01; record what was used in STATUS).
- Ollama runs locally. The owner's Odysseus server runs on the same laptop and over Tailscale (details in HANDOVER.md).

## Talking to the owner

Write plainly and briefly. Say what changed, what you verified and how, and what's uncertain. Don't claim something works unless you ran it. Example sentences in UI copy or docs stay generic, never personal.
