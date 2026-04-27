# Testownik

Testownik is a desktop app (Tauri + React + TypeScript) for practicing question bases, tracking progress, importing legacy/new base formats, and exporting bases for Anki.

## Features

- Select and solve question bases with per-base progress tracking
- Import old `.txt` bases and new `.json` bases
- Edit questions/answers directly in the app
- Export a selected base to Anki-friendly output (text + media)

## Development

### Requirements

- Node.js 20+
- Rust stable toolchain
- Tauri build prerequisites for your OS

### Run locally

```bash
npm ci
npm run tauri dev
```

### Build frontend

```bash
npm run build
```

## GitHub release workflow

This repository includes `.github/workflows/release.yml` to build unsigned release artifacts for:

- Linux (`appimage`, `deb`, `rpm`)
- Windows (`nsis`)

### How to cut a release

1. Keep versions aligned in:
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
2. Create and push a tag like `v0.1.0`.
3. GitHub Actions runs the **Release** workflow and creates/updates a draft release with build artifacts.
4. Review the draft release and publish it from GitHub.

### Manual run

- You can run the workflow manually from the Actions tab, but use a **tag ref** (not a branch).

## Notes

- Artifacts are unsigned in the current setup.
- `old-testownik/` is temporarily ignored in `.gitignore` during migration/release prep.

## Troubleshooting (Linux AppImage)

If AppImage startup fails with `Could not create surfaceless EGL display: EGL_BAD_ALLOC`, run:

```bash
GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 ./Testownik_0.1.4_amd64.AppImage
```

These environment defaults are now applied automatically at Linux startup (with stronger AppImage-safe defaults).
The release workflow also pins Linux AppImage builds to a known-good WebKitGTK 2.44 stack to reduce cross-distro EGL startup issues.
