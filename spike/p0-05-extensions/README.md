# P0-05 spike: Chrome style without Chrome's UI, and MV3 extensions (Windows)

Throwaway code: the P0-02 window plus `--toolbar=none|location|normal` (`BrowserViewDelegate::chrome_toolbar_type`) and `--then=<url>` (a browser-initiated navigation 4 s after the first page). Chrome switches such as `--load-extension` pass straight through to CEF. Builds into `../p0-04-helm/target` (same `cef` crate, so CEF isn't downloaded twice).

```powershell
$env:P005_EXT = "<folder with unpacked-* extension folders>"   # outside the repo
.\run.ps1 -AppArgs "--url=chrome://extensions" -Ext -Fresh -Shot p0-05-x.png
.\run.ps1 -AppArgs "--url=https://example.com --then=chrome-extension://<id>/popup.html" -Ext -Shot p0-05-y.png
```

The extensions were fetched as CRX3 from Google's update service (`clients2.google.com/service/update2/crx?...prodversion=156.0.8078.100`), unpacked (header stripped, zip extracted with path checks) outside the repo. Versions: uBlock Origin Lite 2026.1116.1650, Bitwarden 2026.10.2, Dark Reader 4.9.135. All three are Manifest V3 with a background service worker.

## Results (Windows 11, CEF 156.0.31 / Chromium 156.0.8078.100, Chrome style, Views)

| What | Result | Evidence |
|---|---|---|
| Chrome toolbar / tab strip in our window | **None by default.** Even `--toolbar=normal` shows nothing: the CEF header says the type only makes a toolbar *available* via `BrowserView::GetChromeToolbar()` for the app to place | `../shots/p0-05-toolbar-normal.png`, `p0-05-toolbar-location.png` |
| `chrome://extensions` page | Works | `../shots/p0-05-extensions-page.png` |
| Unpacked (`--load-extension=a,b,c`) | All three load and are enabled. Must be passed on every launch | same |
| Chrome Web Store page | Page loads (after Google's EU consent page, whose text rendered invisible here) with a "Switch to Chrome to install extensions and themes" banner, **but "Add to Chrome" works**: Chrome's own install prompt ("Add 'Dark Reader'? Read and change all your data on all websites") appears, and after "Add extension" the profile holds it as a store install (`location=1`, `from_webstore=true`, files in `Default\Extensions`) | `p0-05-webstore.png`, `p0-05-webstore-add.png`, `p0-05-webstore-installed.png` |
| Background service worker | Running for all three (`chrome://serviceworker-internals`) | `p0-05-service-workers.png` |
| Content scripts | Work: Dark Reader darkens Wikipedia | `p0-05-darkreader-content-script.png` |
| Network blocking (declarativeNetRequest) | Works: with uBOL, navigating to `pagead2.googlesyndication.com/pagead/js/adsbygoogle.js` ends on uBOL's `strictblock.html`; without extensions the request is not intercepted (`ERR_ABORTED`, a script URL at top level) | run log in STATUS |
| Options page | Works when opened as a tab (`chrome-extension://<id>/dashboard.html`) | `p0-05-ublock-options.png` |
| Popup | **No toolbar, so no action button to open it.** The popup page works when opened as a tab (`chrome-extension://<id>/popup/index.html`). Opening it as a real anchored popup needs our own button (P6-02); the API for that is *unverified* | `p0-05-bitwarden-popup-as-tab.png` |
| Start URL `chrome-extension://...` | Fails with `ERR_BLOCKED_BY_CLIENT` when used as the very first URL (extensions not registered yet); works a few seconds later | run log |

## Problems to solve before shipping extensions (written up for the owner)

1. **Extension-opened tabs open a native Chrome window.** Dark Reader opens its help page on install; CEF shows it in a full Chrome browser window (tab strip, toolbar, extensions menu, "Action required" profile button), outside our UI. We need to catch new tabs/windows created by extensions (`chrome.tabs.create`, `windows.create`) and put them in our Trail. Which CEF callback sees them (`LifeSpanHandler::on_before_popup`, `BrowserViewDelegate::on_popup_browser_view_created`, Chrome commands) is *unverified*.
2. **Chrome's external extensions leak in.** Chrome style reads the Windows registry keys that other programs use to register extensions for Google Chrome (`Software\Google\Chrome\Extensions`). In this test one such extension was listed (disabled) and its files were **downloaded from the store into our profile**. Our browser must turn these sources off. How (a switch, a policy or a CEF setting) is *unverified*.
3. **Extensions loaded with `--load-extension` don't persist**: they're passed on every launch. "Load unpacked" from `chrome://extensions` in developer mode was not tried.
4. Q-04 is still open: the store says "Switch to Chrome" but installing works. Whether Google's terms allow this for a non-Google browser was not checked.
5. Google's consent page text drew invisible (dark text on our dark colour scheme, apparently). Watch for pages that assume a light default.

Linux: not tested.
