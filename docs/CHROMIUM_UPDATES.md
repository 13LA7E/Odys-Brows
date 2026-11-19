# Chromium / CEF Maintenance Routine (P9-04)

Standard Operating Procedure for upgrading Chromium / CEF dependencies to incorporate upstream security patches and browser engine improvements.

---

## 1. Upstream Releases Cadence

- **Chromium Major Releases:** Every 4 weeks on upstream Google Chrome milestones.
- **Security Point Releases:** Weekly or bi-weekly for zero-day vulnerabilities (CVEs).
- **CEF Builds:** Maintained at `https://cef-builds.spotifycdn.com/index.html` and tracked on crates.io via `cef` crate.

---

## 2. Upgrade Routine

1. **Check latest published CEF crate version:**
   ```powershell
   cargo search cef
   ```
2. **Update dependency pin in `Cargo.toml`:**
   In root `Cargo.toml`:
   ```toml
   cef = { version = "=156.x.y", default-features = false }
   ```
3. **Verify API compatibility:**
   - Run `cargo check --workspace`
   - If callback signatures or delegate types changed, update `app/engine-cef/src/lib.rs`.
4. **Run test suite & verify:**
   ```powershell
   powershell -ExecutionPolicy Bypass -File tools\check.ps1
   ```
5. **Run benchmarks to detect performance regressions:**
   ```powershell
   python tools\bench\bench.py --browser constellations
   ```
6. **Commit & tag:**
   ```powershell
   git commit -m "chore(engine): upgrade CEF to 156.x.y"
   ```

---

## 3. Automated Monitoring

A GitHub Actions cron workflow (`.github/workflows/cef_check.yml`) runs weekly to query crates.io for newer versions of `cef` and open an alert issue if an upgrade is pending.
