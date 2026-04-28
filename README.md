# Testownik

Testownik to aplikacja desktopowa (Tauri + React + TypeScript) do nauki na bazach pytań: rozwiązywanie testów, śledzenie postępów, import różnych formatów i eksport do Anki. Działa na Windowsie i Linuxie.

## Najważniejsze funkcje

- Rozwiązywanie baz pytań z zapisem postępu per baza
- Import starszego formatu katalogowego (`.txt` + obrazy)
- Import nowszego formatu JSON (`.json`)
- Edycja pytań i odpowiedzi bezpośrednio w aplikacji
- Eksport bazy do formatu Anki (`.txt` + katalog `media/`)

## Wymagania

- Node.js 20+
- Rust (stable)
- Wymagania systemowe Tauri dla Twojego systemu

## Szybki start (development)

```bash
npm ci
npm run tauri dev
```

## Build frontendu

```bash
npm run build
```

## Wspierane formaty baz (schematy)

### 1) Stary format (`.txt`) — import katalogu

Aplikacja akceptuje jeden z poniższych układów (wybierasz katalog główny importu):

```text
wybrany_katalog/
├─ baza/
│  ├─ 001.txt
│  ├─ 002.txt
│  └─ obrazek1.png
├─ 001.txt
├─ 002.txt
└─ nazwa_bazy/
   ├─ baza/
   │  ├─ 001.txt
   │  └─ obrazek2.jpg
   └─ 001.txt
```

Obsługiwane rozszerzenia obrazów: `png`, `jpg`, `jpeg`, `gif`, `webp`.

Schemat pojedynczego pliku pytania `.txt`:

```text
X1010
Treść pytania...
[img]obraz_pytania.png[/img]
a) Odpowiedź A
b) [img]obraz_odpowiedzi_b.jpg[/img]
c) Odpowiedź C
d) Odpowiedź D
```

Znaczenie pierwszej linii (`X...`): po `X` kolejne znaki `1/0` oznaczają poprawność odpowiedzi `a..h` (np. `X1010` => poprawne: `a`, `c`).

### 2) Nowy format (`.json`) — import pliku

Minimalny schemat danych:

```json
{
  "name": "Nazwa bazy",
  "slug": "nazwa-bazy",
  "displayName": "Nazwa Bazy",
  "description": "",
  "questionCount": 1,
  "questions": [
    {
      "id": "q1",
      "question": "Treść pytania",
      "images": ["pytanie.png"],
      "answers": [
        { "key": "a", "text": "Odpowiedź A" },
        { "key": "b", "image": "odp_b.jpg" }
      ],
      "correct": ["a"]
    }
  ]
}
```

Opcjonalne źródła obrazów przy imporcie nowego formatu (względem katalogu z plikiem `.json`):

```text
katalog_z_json/
├─ baza.json
├─ baza/
│  └─ ...
└─ bazy/
   ├─ <slug>/baza/...
   └─ <name>/baza/...
```

### 3) Eksport do Anki

Po eksporcie do wybranego katalogu:

```text
folder_eksportu/
├─ <slug>.txt
└─ media/
   ├─ <slug>_<id_pytania>_<nazwa_obrazu_1>
   └─ <slug>_<id_pytania>_<nazwa_obrazu_2>
```

Plik `<slug>.txt` zawiera karty w formacie `front<TAB>back` (HTML, `<br>`, `<img ...>`), gotowe do importu w Anki.

## Workflow wydań GitHub

Repozytorium zawiera `.github/workflows/release.yml`, który buduje **niepodpisane** artefakty dla:

- Linux (`appimage`, `deb`, `rpm`)
- Windows (`nsis`)

### Jak przygotować wydanie

1. Utrzymaj zgodne wersje w:
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
2. Utwórz i wypchnij tag, np. `v0.1.0`.
3. GitHub Actions uruchomi workflow **Release** i utworzy/zaktualizuje draft wydania.
4. Zweryfikuj draft i opublikuj wydanie w GitHub.

### Uruchomienie ręczne workflow

- Workflow można uruchomić ręcznie z zakładki Actions, ale użyj **refa taga** (nie gałęzi).
