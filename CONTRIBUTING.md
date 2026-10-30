# Contributing to Vane

Thank you for your interest in contributing to Vane! We welcome community contributions to help improve the project.

## How to Contribute

1. **Fork the Repository:** Create a personal fork on GitHub.
2. **Clone the Fork:** Clone the repository locally.
3. **Create a Branch:** Create a branch for your feature or fix using naming conventions:
   - `feat/feature-name` for new features
   - `fix/bug-name` for bug fixes
   - `docs/doc-updates` for documentation changes
4. **Implement & Test:** Make your changes, compile, and pass all verification gates locally.
5. **Commit Changes:** Use semantic commit messages (see guidelines).
6. **Submit a PR:** Push changes to your fork and submit a Pull Request to Vane's `main` branch.

---

## Coding Standards & Architecture Rules

### Frontend (TypeScript + React 18 + Zustand + Vite)
- Use functional React components and custom hooks.
- Style components using CSS Modules (`*.module.css`) to avoid global namespace collisions.
- Ensure strict TypeScript typing (`npx tsc --noEmit` must pass with zero errors; never use `any`).
- Route all system interactions through typed Tauri IPC wrappers.

### Backend (Rust + Tauri v2)
- Avoid `unwrap()` or `expect()` in production runtime paths; return structured `Result<T, VaneError>` types.
- **No Shell Interpolation:** Never execute system commands via `sh -c` or string concatenation. Pass positional argument vectors directly to `Command` or the elevated runner (`pkexec`).
- **Privilege Isolation:** On Linux, `cap_net_admin,cap_net_raw+ep` capabilities apply strictly to the isolated `nfqws` binary — never to the main Tauri executable.
- **Artifact Integrity:** Any bundled binary must be verified by `Sha256ArtifactIntegrityVerifier` immediately prior to process spawn (TOCTOU defense).

---

## Commit Message Guidelines

We follow standard Conventional Commits formatting:
- `feat: add new proxy mode` (new features)
- `fix(dns): enforce functional UDP/TCP readiness checks` (bug fixes)
- `docs: update setup guidelines` (documentation changes)
- `refactor(engine): isolate firewall transaction state` (refactoring code)
- `test: add validate_preset_args unit tests` (adding tests)

---

## Local Development & Verification Gates

### 1. Prerequisites
Ensure you have Node.js (v22 LTS recommended), npm, and the Rust stable toolchain installed.

### 2. Run in Development Mode
```bash
npm install
npm run tauri dev
```

### 3. Mandatory Verification Gates
Before opening a Pull Request, verify that all CI gates pass with zero warnings or errors:

```bash
# 1. Frontend typecheck, unit tests, and production bundle
npx tsc --noEmit
npm test
npm run build

# 2. Backend formatting, strict clippy lints, and test suite
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```
