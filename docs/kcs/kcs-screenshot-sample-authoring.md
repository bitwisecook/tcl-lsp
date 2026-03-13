# KCS: Authoring screenshot samples

## Goal

Keep screenshot scene source files easy to edit without touching TypeScript, while giving deterministic cursor placement for captures.

## Storage location

- Store screenshot scene source files in `samples/for_screenshots/`.
- Keep one sample file per scene intent (or per before/after variant when needed).
- Use `.tcl` for Tcl scenes and `.irul` for iRule scenes.

## Cursor marker contract

The screenshot loader strips marker lines before opening the document and converts them into editor cursor positions.

Use these marker comments in sample files:

```tcl
set a 1
#   ^--- cursor
```

- `^--- cursor`: place the cursor on the previous line at the `^` column.

```tcl
# <<--- cursor on left margin
```

- `<<--- cursor on left margin`: place the cursor on the next line at column `0`.

```tcl
# `--- cursor one in from left margin
```

- `` `--- cursor one in from left margin``: place the cursor on the next line at column `1`.

## Scene wiring contract

- `editors/vscode/src/screenshotDemo.ts` must load scene sources via `loadScreenshotSample(...)`.
- Inline Tcl/iRule source literals in `screenshotDemo.ts` are not allowed.
- Scene setup can still transform sample content (format/fix/optimise), but the baseline source must come from `samples/for_screenshots/`.

## Naming guidance

- Prefer scene-number prefixes to match capture outputs:
  - `03-completions.tcl`
  - `05-security-taint-before.irul`
  - `05-security-taint-after.irul`
- Use explicit suffixes such as `-before`, `-after`, `-long`, `-fallback` where relevant.

## Editing workflow

1. Edit the sample file in `samples/for_screenshots/`.
2. Keep exactly one cursor marker per sample unless a scene intentionally overrides cursor placement in TypeScript.
3. Re-run `make screenshots` and validate the resulting image.

## Failure modes

- Marker typo (`cursor` text missing) -> no parsed cursor; scene fallback cursor is used.
- Marker left in output content -> parser regression in `parseScreenshotSample`.
- Scene still uses inline source -> sample edits do not affect captures.

## File-path anchors

- `editors/vscode/src/screenshotDemo.ts`
- `samples/for_screenshots/`
