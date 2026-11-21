# Updates (AUDIT-23, EDGE_CASES §13)

How Constellations finds, checks and installs a new version. Built and tested (see PHASES, AUDIT-23).

## What the user sees

Settings > About: **Check for updates**, a switch **Check automatically** (a quiet look about 20 seconds
after start; it says nothing unless there is a newer version) and **Update server** (empty = the
official one). When a version is found: its number and notes, **Download and check**, a progress
percentage, then **Restart and install**. Private windows never check.

## What is sent

One plain `GET` of the manifest and, if you ask, one of the installer. No identifier, no profile data,
no history; the only things the server can see are your IP address and the program name `Constellations-updater`.

## The manifest (`latest.json`)

Default address: `https://raw.githubusercontent.com/13LA7E/Odys-Brows/Constellations/releases/latest.json`

```json
{
  "version": "0.2.0",
  "notes": "What is new.",
  "pub_date": "2026-12-01T10:00:00Z",
  "platforms": {
    "windows-x86_64": { "url": "https://.../Constellations-Setup-0.2.0.exe", "sha256": "<64 hex>", "size": 123456789, "signature": "<base64 Ed25519>" },
    "linux-x86_64":   { "url": "https://.../Constellations-0.2.0-x86_64.AppImage", "sha256": "...", "size": 1, "signature": "..." }
  }
}
```

The signature is made over `constellations-update/v1\n<version>\n<platform>\n<sha256>\n<size>` with the
release key. Because the file's hash and size are in the signed text, a swapped file, a changed size or a
changed version all fail.

## The checks (all in `app/shell/src/updater.rs`, all tested)

1. The manifest must be https (a local test server is allowed in **debug builds only**), at most 256 KB.
2. The version must be newer than ours (`1.0.0` beats `0.99.9`; a release beats its own `-beta`).
3. The platform entry must exist, with a 64-hex hash, a sensible size (at most 700 MB) and an https address.
4. The signature must verify with the **public key built into the program**. If not, nothing is offered.
5. The download must be exactly the signed size and hash; redirects must stay on allowed addresses. A file
   that does not match is **deleted** and the running browser is untouched.
6. Only then **Restart and install** starts the installer (Windows: NSIS setup with `/S`, started after the
   browser has closed; Linux: the AppImage). If the file is gone, nothing happens.

## Making a release (the owner)

```
python tools/updater/sign.py --make-key            # once; prints the public key (already built in)
python tools/updater/sign.py --version 0.2.0 --notes "What is new" \
    --windows dist/Constellations-Setup-0.2.0.exe \
    --windows-url https://github.com/13LA7E/Odys-Brows/releases/download/v0.2.0/Constellations-Setup-0.2.0.exe \
    --out releases/latest.json
```

Upload the installer to the GitHub release, commit `releases/latest.json` to the `Constellations` branch.

**The private key** is at `C:\Users\13la7\.constellations\update-signing-key.pem` (outside the repository,
made on 21 Nov 2026). **Back it up somewhere safe and never commit it.** Lost: no installed copy can be
updated again (you would ship a new build with a new public key by hand). Leaked: anyone could send updates;
make a new key, ship a build with the new public key, and tell people to reinstall.

## Limits

- The installer is not code-signed yet (Q-02), so Windows SmartScreen may warn on first run. This is separate
  from our own signature, which is what protects the update channel.
- Linux: the AppImage path is written but untested (no Linux machine).
- Delta updates are not done; every update is the full installer.
