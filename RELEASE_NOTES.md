# v1.10.3

## New Features

- **Tcl 9 `const` command** — Registry entry for `const varName value`
  with hover docs, arity validation, and side-effect metadata.
  Re-applying `const` to an existing constant is a silent no-op;
  applying it to an existing non-constant variable raises.
- **`[global]` and `[project]` config sections** — The XDG `config.ini`
  now reads `dialect`, `extraCommands`, and `libraryPaths` from a
  `[global]` section, and per-project `.tcl-lsp.ini` reads them from
  `[project]`. The section name encodes which precedence layer the file
  occupies, mirroring the location-based safeguard documented in
  `docs/design/contracts/config-precedence.md`.
- **`tcl-lsp.getEffectiveConfig` LSP command** — New
  `workspace/executeCommand` that returns the resolved feature config
  (dialect, library paths, feature toggles, optimiser/shimmer state,
  disabled diagnostics, …) for a given URI. Lets editor clients poll
  for `workspace/configuration` changes to settle instead of sleeping
  on wall-clock time.

## Improvements

- **WASM proc params source preserved** — The codegen prologue now
  stashes the raw bytes of each proc's params spec via a new
  `proc_set_params_source_raw` host import, so `info args` and
  `info default` materialise a fresh `TclObj` on demand for
  AOT-compiled procs. Pointer-only ABI, so no `tcl_obj_retain` cascade
  at module init.
- **VS Code configuration-settings tests** rewritten to poll
  `getEffectiveConfig` for the toggle they care about, removing the
  flaky time-based waits that were hiding intermittent races.
- **Codegen artefacts regenerated** — `_registry_data.tcl`,
  `_mock_stubs.tcl`, and the Zed command JSON now include the new
  `const` entry.

## Bug Fixes

- **`tcl-lsp.getEffectiveConfig` no longer fails on every call** — The
  handler's first parameter used PEP 604 union syntax (`str | None`),
  which trips a `TypeError` inside pygls's
  `issubclass(ptype, LanguageServer)` first-parameter check. Changed
  to `str = ""` to match the other handlers in `lsp/commands.py`; the
  body already treats falsy `uri` as the workspace-fallback case.
- **`proc_set_params_source_raw` permitted in pure-arithmetic import
  allowlist** — Without this, the strictness test fired for every WASM
  module that defined a proc with arguments.
