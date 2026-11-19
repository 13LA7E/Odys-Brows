# Code Signing Guide (P9-02)

Instructions and best practices for signing Constellations release binaries on Windows and Linux.

> **CRITICAL RULE (AGENTS.md):** Never commit private keys, passwords, certificates or secrets to git. The owner enters passwords only into encrypted GitHub Actions Secrets.

---

## 1. Windows Authenticode Signing

Windows SmartScreen requires code signing to build download reputation and prevent security warnings on first launch.

### Recommended Options

1. **Azure Trusted Signing (Recommended for CI):**
   - Cloud-based signing service integrated with Microsoft Identity.
   - Eliminates the need for physical hardware USB dongles.
   - GitHub Actions integration via `azure/trusted-signing-action`.

2. **Standard / EV Hardware Token:**
   - Physical YubiKey or Safenet eToken with Signtool.
   - Requires manual signing on the owner's workstation or a self-hosted runner.

### Signing Command (Workstation)

```powershell
& "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe" sign `
    /tr http://timestamp.digicert.com `
    /td sha256 `
    /fd sha256 `
    /a `
    target\release\constellations.exe
```

### GitHub Actions Secrets Setup

In GitHub repo settings (`Settings -> Secrets and variables -> Actions`):
- `AZURE_CLIENT_ID`: App registration client ID
- `AZURE_TENANT_ID`: Microsoft Entra tenant ID
- `AZURE_CLIENT_SECRET`: App client secret
- `SIGNING_ENDPOINT`: `https://eus.codesigning.azure.net/`
- `SIGNING_ACCOUNT`: Account name
- `SIGNING_PROFILE`: Certificate profile name

---

## 2. Linux Signing

### AppImage

AppImages support embedded digital signatures verified by `appimaged` or `AppImageUpdate`:

```bash
gpg --export --armor > pubkey.asc
appimagetool -s --sign-key=<KEY-ID> AppDir Constellations-x86_64.AppImage
```

### Debian (.deb) Package

```bash
dpkg-sig --sign builder -k <KEY-ID> constellations_0.1.0_amd64.deb
```
