# P0-02 spike: one CEF window on Windows

Minimal Rust app on `tauri-apps/cef-rs`: one CEF Views window, Chrome runtime style (D-003), loading https://example.com. Throwaway code.

## Versions (exact)

- `cef` crate `=156.7.0+156.0.31` (crates.io, repo `tauri-apps/cef-rs`); `cef-dll-sys` 156.7.0+156.0.31; `download-cef` 3.0.1.
- CEF `156.0.31+ge603d97+chromium-156.0.8078.100` (from `include/cef_version.h`), Windows x64 **minimal** distribution, downloaded by `cef-dll-sys`'s build script into `target/.../build/cef-dll-sys/<hash>/out/cef_windows_x86_64`.

## Build and run (PowerShell)

```powershell
cd spike\p0-02-cef-window
cargo build
$cef = (Get-ChildItem target\debug\build\cef-dll-sys -Recurse -Filter libcef.dll | Select-Object -First 1).DirectoryName
$env:PATH = "$cef;$env:PATH"
.\target\debug\p0-02-cef-window.exe                 # example.com
.\target\debug\p0-02-cef-window.exe --url=https://...  # any URL
```

`shot.ps1 -ProcessId <id> -Out <png>` saves a screenshot of that process's window with `PrintWindow` (never the whole screen, so nothing else on the desktop is captured).

## Findings

- With CEF Views + `RuntimeStyle::CHROME`, the window has **no Chrome toolbar or tab strip** by default: only a thin title strip with an icon and a close button (`../shots/p0-02.png`). Input for P0-05.
- **A Windows app manifest is required.** Without `supportedOS` for Windows 10/11, the GPU process crashed on start (exit code -2147483645 = 0x80000003, a breakpoint/CHECK) and Chromium fell back to software. `build.rs` embeds `app.manifest` with the MSVC linker (`/MANIFEST:EMBED /MANIFESTINPUT:`), no extra crate. After that, no GPU errors.
- Closing the window (WM_CLOSE) ends all CEF processes (6 to 8) in about 0.3 s.
- A second copy using the same `root_cache_path` fails `initialize` cleanly; the spike prints a message and exits with code 2. The real app needs single-instance hand-off (EDGE §1).
- An unreachable host shows Chrome's own error page (`../shots/p0-02-bad-url.png`) and our `on_load_error` logs `ERR_NAME_NOT_RESOLVED`.
- Console noise that is harmless: `external_registry_loader_win.cc: Missing value path for key Software\Google\Chrome\Extensions\...` (Chrome style reads Chrome's external-extension registry keys; the keys on this PC belong to an installed Chrome).
- Sandbox is off (`no_sandbox = 1`, crate feature `sandbox` disabled). With the sandbox on Windows, cef-rs needs its `bootstrap.exe` + DLL layout (`bundle-cef-app`); decide in P0-08.
