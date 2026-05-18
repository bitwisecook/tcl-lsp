# v1.10.5

## New Features

- **Tcl 9.0 command coverage.** New registry entries and codegen support
  for `lpop`, `foreachline`, `readfile`, `writefile`, `tcl::idna`,
  `tcl::process`, and `fconfigure` option additions, with per-option
  dialect gating so 8.5 / 8.6 / 9.0 / iRules / iApps each see only the
  surface they actually support (#433).
- **W003 / W004 diagnostics.** Two new analyser warnings surface
  iRules-specific authoring issues, plumbed through the diagnostic
  manifest and into the VS Code, JetBrains, Zed, and emacs catalogs.
- **iApp diagnostics.** A new `iapp_diagnostics` pathway flags
  iApp-template authoring problems that previously went unreported.
- **Domain-aware analysis checks.** `core/analysis/checks/_domain.py`
  hosts cross-cutting checks that depend on combined command +
  dialect + side-effect context.
- **Option-dialect auditing.** `scripts/audit_option_dialects.py`
  reports option-coverage gaps across dialects so registry edits stay
  in sync as Tcl 9 surface grows.

## Improvements

- **Tighter command-registry models** for `chan`, `clock`, `encoding`,
  `exec`, `interp`, `lsearch`, `lsort`, `regsub`, `socket`, `source`,
  `switch`, and `vwait`, plus updates to the tcllib `fileutil`,
  `math::statistics`, `mime`, and `textutil` modules.
- **cmdAH cascade and `info` introspection gaps closed (#430)** so
  hover, completion, and go-to-definition now resolve commands that
  previously fell through.
- **Snippet templates** refreshed with new entries.
- **Tail-call and DCE optimiser passes** updated alongside the
  expanded command surface.
- **Installer smoke test** factored out of the release skill into a
  standalone `scripts/smoke_installer.sh`.

## Bug Fixes

- **`probePython` no longer canonicalises bare `PATH` names**, so
  shimmed interpreters (pyenv, asdf, mise) resolve to the active
  shim rather than the underlying real binary.
- **`/etc/os-release` symlinks are followed** before the ownership
  check, fixing distro detection on systems where the file is a
  symlink into `/usr/lib` (NixOS, immutable distros).
- **Dialect-detection extension tests** get a 15 s → 30 s
  `waitForCompletions` budget, eliminating a flake on the Linux CI
  runner where the dialect-change config notification could arrive
  after the previous window expired (#435).

## Internal

- **Tag-only release flow.** Every version literal in the tree now
  derives from the latest annotated git tag (via `hatch-vcs` for the
  Python wheel and via the Makefile + `git describe` for every editor
  build). Cutting a release is `git tag -a vX.Y.Z … && git push
  origin vX.Y.Z` — no source-file bumps, no commit on `main` (#436).
- **JetBrains build** no longer mutates `gradle.properties`; the
  version comes from the `RELEASE_VERSION` env var the Makefile sets
  (#436).
- `ty` pinned to `==0.0.37`; previously the range allowed a stale
  `.venv` to disagree with CI about whether `# ty: ignore[...]`
  directives were redundant (#435).
- `@typescript-eslint/eslint-plugin` and `parser` bumped to `^8.59.4`,
  the first release whose `typescript` peer accepts the `^6.0.3` we
  now ship (#435).
- Dependency refresh: TypeScript 6.0, Gradle, pinned GitHub Actions
  versions (#434).
- CI: corrected `ossf/scorecard-action` SHA pin and bumped
  `actions/checkout` to v5 (#431).
