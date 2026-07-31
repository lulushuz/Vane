# Code-Signing & Updater Secret Configuration Contract

## Authoritative Secret Names

The following GitHub Actions secrets are defined for automated signing workflows:

| Secret Name | Purpose | Scope |
| :--- | :--- | :--- |
| `TAURI_SIGNING_PRIVATE_KEY` | Tauri updater bundle signing private key (Ed25519) | GitHub Protected Environment |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for Tauri updater private key | GitHub Protected Environment |
| `WINDOWS_SIGNING_CERTIFICATE_BASE64` | Production PFX code-signing certificate (Base64) | GitHub Protected Environment |
| `WINDOWS_SIGNING_CERTIFICATE_PASSWORD` | Password for Windows code-signing certificate | GitHub Protected Environment |
| `WINDOWS_SIGNING_TIMESTAMP_URL` | RFC 3161 compliant timestamp server URL | Environment Variable / Secret |

## Security Rules
1. **Repository Exclusion:** Secrets MUST NEVER be committed to Git.
2. **Access Control:** Secrets are accessible only within protected GitHub Release environments requiring maintainer review. Pull Request workflows have zero access to signing keys.
3. **Log Protection:** Workflows redact all signing output; temporary certificate files are destroyed in `always()` cleanup steps.
4. **Timestamp Verification:** All Windows executables and NSIS installers MUST include valid RFC 3161 timestamps to ensure longevity after certificate expiration.
