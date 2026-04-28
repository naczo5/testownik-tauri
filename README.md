# Testownik

Testownik is a desktop application (Tauri + React + TypeScript) for practicing question bases: solving tests, tracking progress, importing different base formats, and exporting to Anki. It runs on Windows and Linux.

## Key features

- Solve question bases with per-base progress tracking
- Import legacy directory format (`.txt` + images)
- Import newer JSON format (`.json`)
- Edit questions and answers directly in the app
- Export a base to Anki format (`.txt` + `media/` directory)

## Requirements

- Node.js 20+
- Rust (stable)
- Tauri build prerequisites for your operating system

## Quick start (development)

```bash
npm ci
npm run tauri dev
```

## Frontend build

```bash
npm run build
```

## Supported base formats (schematics)

### 1) Legacy format (`.txt`) — directory import

The app accepts one of the following layouts (you select the top-level import directory):

```text
selected_directory/
├─ baza/
│  ├─ 001.txt
│  ├─ 002.txt
│  └─ image1.png
├─ 001.txt
├─ 002.txt
└─ base_name/
   ├─ baza/
   │  ├─ 001.txt
   │  └─ image2.jpg
   └─ 001.txt
```

Supported image extensions: `png`, `jpg`, `jpeg`, `gif`, `webp`.

Single `.txt` question file format:

```text
X1010
Question content...
[img]question_image.png[/img]
a) Answer A
b) [img]answer_b_image.jpg[/img]
c) Answer C
d) Answer D
```

Meaning of the first line (`X...`): after `X`, each `1/0` marks correctness for answers `a..h` (e.g. `X1010` => correct: `a`, `c`).

### 2) New format (`.json`) — file import

Minimal data schema:

```json
{
  "name": "Base Name",
  "slug": "base-name",
  "displayName": "Base Name",
  "description": "",
  "questionCount": 1,
  "questions": [
    {
      "id": "q1",
      "question": "Question content",
      "images": ["question.png"],
      "answers": [
        { "key": "a", "text": "Answer A" },
        { "key": "b", "image": "answer_b.jpg" }
      ],
      "correct": ["a"]
    }
  ]
}
```

Optional image source locations during new-format import (relative to the `.json` file directory):

```text
json_directory/
├─ baza.json
├─ baza/
│  └─ ...
└─ bazy/
   ├─ <slug>/baza/...
   └─ <name>/baza/...
```

### 3) Export to Anki

After export to a selected directory:

```text
export_directory/
├─ <slug>.txt
└─ media/
   ├─ <slug>_<question_id>_<image_name_1>
   └─ <slug>_<question_id>_<image_name_2>
```

The `<slug>.txt` file contains cards in `front<TAB>back` format (HTML, `<br>`, `<img ...>`), ready to import into Anki.

## GitHub release workflow

This repository includes `.github/workflows/release.yml`, which builds **unsigned** release artifacts for:

- Linux (`appimage`, `deb`, `rpm`)
- Windows (`nsis`)

### How to cut a release

1. Keep versions aligned in:
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
2. Create and push a tag, e.g. `v0.1.0`.
3. GitHub Actions runs the **Release** workflow and creates/updates a draft release.
4. Review the draft and publish the release on GitHub.

### Manual workflow run

- You can run the workflow manually from the Actions tab, but use a **tag ref** (not a branch).
