# KCS: feature — Document Links

> **Audience:** User
> **Type:** Functionality

## Summary

Clickable links on the files a script loads: the path argument of
`source`, and the package name in `package require`.

## Applies to

all-editors, analyser

## How to use

- **Editor**: Ctrl+Click a `source` path to open that file in a new tab.
  A `package require` name is underlined and carries a tooltip, but has
  no target to open — the package index is not scanned.
- **Settings**: Toggle with `tclLsp.features.documentLinks`.

## Operational context

A relative path resolves against the document's own directory, and `~`
expands against `$HOME`. A computed path resolves when it is built from
`[info script]`, `[file dirname …]`, `[file join …]`, literal words, and
variables the document assigns exactly once, so the common idiom links:

```tcl
set currentDir [file dirname [info script]]
source [file join $currentDir testUtilities.tcl]
```

Only the file name — `testUtilities.tcl` — is underlined, not the whole
`[file join …]` substitution. The substitution is code, with its own
highlighting; an editor paints a link range in one flat link colour, so
underlining all of it would hide the colouring of `file`, `join`, and
`$currentDir` (issue #775).

## File-path anchors

- `rust/tcl-lsp-core/src/document_links.rs`
- `rust/tcl-compiler/src/auto_path_eval.rs` — the shared path evaluator

## Failure modes

- **No link on a computed path.** The path is outside the evaluator's
  supported subset, so the provider abstains rather than guess a target.
  `file normalize` is a common cause: `set dir [file normalize [file
  dirname [info script]]]` does not fold, so a later `source [file join
  $dir x.tcl]` has no resolvable directory. Assign the directory with
  `[file dirname [info script]]` alone to get the link.
- **No link when the substitution ends in a variable.** `source [file
  join $dir $name]` has no literal word to anchor the link on, so none
  is offered even when the path itself resolves.
- **No link on a relative path in an unsaved file.** There is no
  document directory to resolve against until the file is saved.

## Test anchors

- `rust/tcl-lsp-core/src/document_links.rs` — unit tests
- `rust/tcl-lsp-core/tests/lsp_lens_links_symbols.rs`
- `rust/tcl-lsp-server/tests/e2e/semantic_tokens.rs` — the link range
  never spans more than one semantic token

## Example

In this Tcl file:

```tcl
package require tcltest
set currentDir [file dirname [info script]]
source [file join $currentDir testUtilities.tcl]
source lib/helpers.tcl
```

`tcltest` is underlined with a tooltip, `testUtilities.tcl` and
`lib/helpers.tcl` open their files on Ctrl+Click, and the rest of the
`[file join …]` call keeps its normal highlighting.

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
