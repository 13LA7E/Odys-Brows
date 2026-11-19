# Release Checklist (P9-05)

Mandatory verification checklist before tagging and publishing any beta or production release of Constellations.

---

## 1. Codebase & Integrity

- [ ] `git status` clean with no untracked or uncommitted files.
- [ ] No secrets, keys, or credentials committed (`git log -p -n 20`).
- [ ] `cargo fmt --check` passes with zero formatting diffs.
- [ ] `cargo clippy --all-targets -- -D warnings` completes warning-free.
- [ ] `cargo test --workspace` passes 100% of unit and integration tests.
- [ ] `node --check ui/*.js` passes on all JavaScript files.
- [ ] Token check passes (zero raw hex colors outside `ui/tokens.css`).

---

## 2. Visual & Functional Quality

- [ ] `python tools/visual_check.py` passes all baseline design boards.
- [ ] Native window clipping properly exposes Notch and Helm above active web content.
- [ ] New tab (+) button renders centered and symmetric.
- [ ] Space switching and tab tree hierarchies restore properly after restart.
- [ ] Ad and tracker shields block requests and show live count in the notch.
- [ ] Tab sleeping correctly idles background tabs after timeout.
- [ ] Passwords saved via DPAPI encrypt and reveal securely.
- [ ] AI assistant connects to configured local or cloud provider and streams answers into Helm answer card.

---

## 3. Performance & Efficiency

- [ ] Rerun memory benchmark: `python tools/bench/bench.py --browser constellations`
- [ ] Verify private bytes across processes is logged in `docs/BENCHMARKS.md`.
- [ ] Background tab sleeping drops memory by at least 30% with 20 tabs open.

---

## 4. Packaging & Artifacts

- [ ] Windows installer generated: `powershell -ExecutionPolicy Bypass -File tools\installer\build_installer.ps1`.
- [ ] Executables Authenticode signed (or Azure Trusted Signing executed in CI).
- [ ] Linux AppImage and .deb generated: `bash tools/installer/build_linux.sh`.
- [ ] SHA-256 digests and update manifest generated in `tools/updater`.
- [ ] GitHub Release tagged on `Constellations` branch with release notes and installers uploaded.
- [ ] Deliver first beta binary to the owner for validation.
