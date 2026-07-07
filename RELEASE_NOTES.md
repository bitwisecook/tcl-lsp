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
- **Reports carry provenance.** Generated reports are stamped with the git
  commit hash and gain print headers/footers for cleaner hard copies.

## Improvements

- **Much broader registry coverage.** Comprehensive tcllib and Tk command
  coverage (including ttk and ticklecharts), plus BIG-IP config defaults,
  object specs, cross-references, and event facts — sharpening completion,
  hover, and diagnostics across Tcl, Tk, and iRule/BIG-IP code.
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

## Release-pipeline changes

- VS Code pre-releases publish through a PAT-backed, approval-gated CI job
  (the keyless Azure/OIDC path was rolled back after proving unreliable).
- JetBrains pre-releases now publish to the Marketplace **eap** channel from an
  approval-gated CI job, matching the VS Code pre-release track.
- Removed the retired Zig WASM runtime and its build infrastructure.

## Using this alpha

Behaviour should match the 1.x stable line. Where it does not, that is a bug —
please file it and note that you are on the 2.x pre-release.
