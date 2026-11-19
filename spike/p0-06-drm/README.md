# P0-06 spike: DRM (Widevine) on Windows

Uses the P0-05 spike binary (`../p0-05-extensions`, Chrome style, CEF 156.0.31 / Chromium 156.0.8078.100, standard CEF "minimal" distribution from cef-rs). Pages:

- `eme-probe.html`: codec support (MSE) and EME key systems. Open it over http(s), not `file://`: EME isn't allowed on opaque origins (the first run from `file://` said so).
- `play.html`: Shaka Player 4.12.6 (jsDelivr) plays two public Widevine demo assets with Shaka's public license proxy (`cwip-shaka-proxy.appspot.com/no_auth`) and reports codec, time and errors.

```powershell
cd spike\p0-06-drm; python -m http.server 8765 --bind 127.0.0.1     # localhost counts as a secure context
cd ..\p0-05-extensions; .\run.ps1 -AppArgs "--url=http://localhost:8765/play.html" -Shot p0-06-x.png -Wait 45
```

## Results (Windows 11, 19 Nov 2026)

| Step | Result | Evidence |
|---|---|---|
| Widevine component on a fresh profile | Registered in `chrome://components` as "Widevine Content Decryption Module", version 0.0.0.0, status New. No command-line switch was needed | `../shots/p0-06-components-end.png` |
| Getting the CDM | After "Check for update" it downloaded within a minute: **version 4.10.3112.0**, `WidevineCdm/4.10.3112.0/_platform_specific/win_x64/widevinecdm.dll` (+ `.sig`) in the profile folder. Other components updated in the same minute, so the automatic updater was running too; whether it would have fetched Widevine without the click is *unverified* | `p0-06-components-widevine.png` |
| Key systems seen by pages | `com.widevine.alpha`: schemes `cenc`, `cbcs`; robustness `SW_SECURE_CRYPTO`, `SW_SECURE_DECODE` (software, L3); `persistentState: false`. PlayReady and FairPlay: none | `p0-06-shaka-support-drm.png` (Shaka support page) |
| Codecs | **H.264 and AAC: not supported** (the stock CEF build has no proprietary codecs). HEVC, AC-3/E-AC-3, MP3-in-MP4: no. VP9, AV1, Opus, FLAC: yes | `p0-06-eme-probe-default.png`, `p0-06-shaka-support.png` |
| Widevine playback | **Works with VP9**: Angel One (multicodec) played VP9 768x576, Sintel played VP9 1280x546, both `com.widevine.alpha`, time advancing (7.3 s and 7.5 s after 8 s) | `p0-06-widevine-playback.png` |

## What this means

- Widevine works in a CEF Chrome style app on Windows, out of the box, through Chrome's component updater.
- Services that offer only H.264/AAC streams won't play, with or without DRM. Getting H.264/AAC needs a CEF build with proprietary codecs (`proprietary_codecs=true ffmpeg_branding=Chrome`), which brings patent-licensing questions. That's an owner decision (Q-08).
- Commercial streaming services often also require Verified Media Path (VMP) signing of the browser for Widevine on desktop, plus higher robustness for HD. Not tested here; *unverified* for this app.

Linux: not tested.
