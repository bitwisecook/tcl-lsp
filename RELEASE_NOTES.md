# v2.1.4

**2.x alpha — pre-release channel.**

Another pre-release on the **2.x** line, where the ongoing Python → Rust
rewrite of tcl-lsp ships its alphas. It is opt-in: install it from the VS Code
Marketplace **pre-release** channel or the JetBrains Marketplace **eap**
channel, or download the pre-release VSIX / plugin / native binaries from this
GitHub release. The stable **1.x** line stays the default for everyone who has
not opted into pre-releases, and a `2.1.x` build never becomes the "latest"
GitHub release or the default Marketplace download.

## New Features

- **Command-option highlighting.** Command options — including TclOO
  object-method options — are now recognised and highlighted from the registry,
  and option/subcommand names may be given by unambiguous prefix abbreviation
  the way Tcl itself accepts them.
- **By-reference and array-element variable highlighting.** Variables and
  commands passed by reference are inferred from their general argument roles
  and highlighted accordingly, and literal array-element write targets —
  `set arr(key) 1`, `incr count(hits)`, `unset arr(key)` — now highlight as
  whole-word variables to match their reads (a computed subscript like
  `arr($i)` still keeps its inner `$i` as a distinct token).
- **Native BIG-IP report generator.** The report generator is now backed by a
  native Rust CLI alongside the Python backend, both sharing one templating
  layer and emitting the same single-file report, and it ships as a release
  artefact. The interactive report also gains a clearer theme toggle (distinct
  auto/light/dark glyphs) and object-popover titles that deep-link to the
  object's own listing row.
- **Reports carry provenance.** Generated reports are stamped with the git
  commit hash and gain print headers/footers for cleaner hard copies.

## Improvements

- **Much broader registry coverage.** Comprehensive tcllib and Tk command
  coverage (including ttk and ticklecharts), plus BIG-IP config defaults,
  object specs, cross-references, and event facts — sharpening completion,
  hover, and diagnostics across Tcl, Tk, and iRule/BIG-IP code. The `::html::`
  package table was rebuilt against html 1.6 with correct arities, return
  types, purity flags, and version gating, and a dedicated `report::defstyle`
  spec was added.
- **Cross-file `package require` awareness (fewer false W120).** A module
  sourced by an entry file that runs the requires is no longer flagged for
  W120: the workspace now inherits `package require`s along the reverse
  `source` graph. Optionally, `[project] entryPoints` in `.tcl-lsp.ini`
  declares the entry files explicitly and applies their requires project-wide.
- **Diagram rendering switched to elkjs.** Control-flow and data-flow diagrams
  now lay out with elkjs instead of Mermaid, for more stable, readable graphs.
- **Better tcltest support.** Registry tests migrated and the `tcltest`
  package surface improved.
- **Boolean handling in the syntax layer** for more accurate analysis of
  boolean literals.

## Bug Fixes

- **Object dispatch.** Correctly resolve non-static command and TclOO object
  dispatch so methods on objects are analysed instead of flagged as unknown.
- **Variable highlighting.** `unset` and `global` now highlight every variable
  name in the command, not just the first.
- **Config-file dialect now honoured for `.tcl` files.** The `dialect =` key in
  `.tcl-lsp.ini` / `config.ini` is applied to normally-opened `.tcl` buffers;
  the bare `tcl` language id every editor sends now defers to the per-folder
  override and session default instead of forcing `tcl8.6`. Explicit versioned
  ids and in-source `# tcl-dialect:` directives remain authoritative.

## Release-pipeline changes

- VS Code pre-releases publish through a PAT-backed, approval-gated CI job
  (the keyless Azure/OIDC path was rolled back after proving unreliable).
- JetBrains pre-releases now publish to the Marketplace **eap** channel from an
  approval-gated CI job, matching the VS Code pre-release track.
- BIG-IP report and GitHub Pages builds were reworked around the shared
  `bigip-report-gen` layout, including a fix to the single-file WASM report
  build inputs.
- Removed the retired Zig WASM runtime and the vendored TCL regex build
  infrastructure.

## Using this alpha

Behaviour should match the 1.x stable line. Where it does not, that is a bug —
please file it and note that you are on the 2.x pre-release.
