# KCS: feature — Folding

> **Audience:** User
> **Type:** Functionality

## Summary

Code folding for procs, namespaces, event handlers, and braced blocks.

## Applies to

all-editors, analyser

## How to use

- **Editor**: Click fold markers in the gutter or use Ctrl+Shift+[ to fold.
- **Settings**: Toggle with `tclLsp.features.folding`. Defaults to on and
  deliberately does not inherit `editor.folding` — sticky scroll consumes
  folding ranges even when the folding UI is off (issue #1122).

## Operational context

Folding ranges are computed from the parsed AST, identifying proc bodies, namespace blocks, `when` event handlers, and multi-line braced expressions.

BIG-IP configs (`bigip.conf` and friends) are not Tcl source, so they fold on
their own path: a brace-balanced scan of the stanza tree that folds every
multi-line `{ … }` region at any depth — `ltm virtual … { … }`, the
`profiles` / `records` / `members` blocks nested in it, and the Tcl inside an
`ltm rule` body — plus the same comment-block folds the Tcl provider emits.
Before that split the Tcl brace walk ran over config text and found only
comment blocks, leaving every stanza unfoldable.

In VS Code the folding ranges also drive **sticky scroll** for Tcl code
languages. VS Code picks the sticky model from a fallback chain — outline
model → folding-range provider → indentation heuristic — and a non-empty
outline stops the chain. A Tcl script whose top level is `if` / `for` /
`foreach` has only single-line symbols in its outline, so the outline model
pins nothing and sticky scroll goes blank (issue #1122). The extension
therefore contributes
`"editor.stickyScroll.defaultModel": "foldingProviderModel"` as a
language-scoped default for every Tcl language, which pins the same block
headers folding already knows about. BIG-IP config needs the same default
for a different reason: its outline is a synthesised `module → kind →
object` tree whose module and kind nodes take their selection range from
their first child, so the outline model pins the first `ltm` stanza in the
file no matter where the cursor is. It is a default, so a user setting for
`editor.stickyScroll.defaultModel` still wins.

Because sticky scroll rides on folding, the folding response contract
matters beyond the fold gutter: VS Code (≥1.105) accepts an *empty*
folding answer as a valid, terminal sticky model — the chain never falls
through to the indentation heuristic and sticky scroll stays blank. The
server therefore never returns an empty list from
`textDocument/foldingRange`: when the feature is disabled, the document
is unknown, or there is genuinely nothing to fold, it returns `null`,
which lets the folding UI fall back to indentation folding and sticky
scroll fall through to its indentation model. The server also sends
`workspace/foldingRange/refresh` once after the initial workspace scan,
so a sticky model computed before the provider registered is recomputed
against real data.

The version-pinned dialect languages (`tcl8.4`, `tcl8.5`, `tcl9.0`, `tcl9.1`)
are the one gap. A language id containing a `.` cannot be used as a
configuration override identifier — VS Code splits `[tcl8.4]` on the dot while
building the default-configuration value tree, throws, and drops every
remaining override in the block — so those four keep the outline model.

## File-path anchors

- `rust/tcl-lsp-core/src/folding.rs` — Tcl folding walk (native server)
- `rust/tcl-lsp-core/src/bigip.rs` — `folding_ranges` (BIG-IP stanza folds)
- `editors/vscode/package.json` — `configurationDefaults` sticky-scroll model per language

## Failure modes

- Folding ranges missing or incorrect after parser changes.
- Sticky scroll blank in VS Code when the folding provider ends up with
  no data — see
  [Sticky scroll shows nothing in VS Code](../kcs-issue-sticky-scroll-shows-nothing.md).

## Test anchors

- `rust/tcl-lsp-core/src/folding.rs` — fold-walk unit tests, EOF/CRLF bounds
- `rust/tcl-lsp-server/tests/e2e/issue1122_sticky_scroll.rs` — null-not-empty contract + sticky candidate bounds
- `rust/tcl-lsp-core/src/bigip.rs` — stanza / embedded-rule / unbalanced-input fold unit tests
- `rust/tcl-lsp-server/tests/e2e/bigip.rs` — `conf_stanzas_and_their_nested_blocks_fold`
- `editors/vscode/src/test/stickyScroll.test.ts` — per-language sticky-scroll defaults

## Screenshots

- `20-folding` — code folded to show structure

![code folded to show structure](../screenshots/20-folding.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
