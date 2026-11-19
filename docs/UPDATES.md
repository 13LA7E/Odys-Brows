# Auto-Update Architecture (P9-03, EDGE_CASES §13)

How Constellations discovers, verifies, and applies application updates without telemetry or silent breakage.

---

## 1. Update Discovery

- **Manifest Endpoint:** `https://raw.githubusercontent.com/13LA7E/Odys-Brows/Constellations/releases/latest.json`
- **Cadence:** Checked on startup or when the user clicks "Check for updates" in Settings -> About.
- **Privacy:** Request includes only standard HTTP GET headers; no user ID, profile data, or browser history is transmitted.

---

## 2. Security & Signature Verification

Before executing any downloaded update payload:
1. The update binary's SHA-256 digest is calculated.
2. The cryptographic signature in the manifest (`minisign` / Ed25519) is verified against the bundled public key hardcoded in the browser.
3. If signature verification fails or the network drops mid-download, the temporary download is purged immediately and the running browser is untouched (EDGE_CASES §13).

---

## 3. Platform Execution

- **Windows:**
  - Setup executable downloaded to temporary cache (`%TEMP%\Constellations-Update.exe`).
  - Verified and executed with silent flags `/S` on browser restart.
- **Linux:**
  - For AppImages: uses `AppImageUpdate` zsync delta updates.
  - For `.deb`: user notified or system package manager handles repository updates.
