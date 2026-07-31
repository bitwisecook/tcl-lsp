# KCS: feature — Document Symbols

> **Audience:** User
> **Type:** Functionality

## Summary

Outline of procs, namespaces, event handlers, variables, and `tcltest` definitions (test cases, constraints, custom match modes) in the current file.

## Applies to

all-editors, MCP, analyser

## How to use

- **Editor**: Ctrl+Shift+O or the Outline panel.
- **MCP**: `symbols` tool — pass source code.
- **Settings**: Toggle with `tclLsp.features.documentSymbols`.

## Operational context

Produces a hierarchical symbol tree with procs nested inside namespaces, variables inside procs, and event handlers (iRules `when` blocks) at the top level.

An iRules `when EVENT { … }` handler is an outline entry of its own kind (LSP `Event`), named for its event and ranged over the whole handler. Like the `tcltest` definitions below it is registry-driven — `when` declares `defines_symbol` naming argument 0 — so `when EVENT priority 500 { … }` and a `when` reached through any of its keyword-tail forms all list identically, while a dynamic `when $evt { … }` is skipped rather than shown as `$evt`.

A body that does **not** open a scope — a `when` handler, a `tcltest::test` case — contributes its definitions to the *enclosing* scope, so the handler and the variables set inside it arrive as siblings with overlapping ranges. The provider re-parents each symbol under the innermost sibling whose range contains it, so the outline is a proper tree (a nested `when` inside a `when` nests too). Documents without that shape — every plain Tcl file, where a scope body *is* a scope — are unaffected.

Commands that *define a named unit* also contribute an outline entry, each under its own kind: `tcltest::test NAME …` (a function-like test case, description shown as detail), `tcltest::testConstraint NAME value` (a constant-kind constraint — only the two-argument setter defines; the one-argument getter is a reference), and `tcltest::customMatch MODE command` (an operator-kind match mode). All forms are recognised whether the command is called qualified or via `namespace import ::tcltest::*`, and a definition inside `namespace eval` nests under it. This is registry-driven: a command declares `defines_symbol` in its [`CommandSpec`](../../design/compiler/command-registry.md) and every symbol consumer (document + workspace symbols, MCP) picks it up generically, so adding the next such command is a spec change, not a compiler edit. The *name* is resolved through the analyser's constant-propagation lattice, so a name written as a literal, a quoted string, or a constant `$var` all resolve; a genuinely dynamic name is skipped rather than shown as raw `$var` text (#790).

A `TclOO` class body contributes its members as children of the class symbol: `method` (instance), `classmethod` and `self method` (class-side, shown with a `classmethod` detail), `constructor`, `destructor`, and `property`. Both spellings of the class-side and visibility wrappers list — the prefix form (`self method make {n} {…}`) and the *block* form (`self { method make {n} {…} }`), and likewise `private method …` / `private { method … }`. The block form is normalised into the prefix form by the definition-body walker from registry data (the definer grammar marks `self` and `private` as wrapper members that also take a bare script block), so both spellings travel one code path and pick up dispatch, hover, and go-to-definition together with the outline entry (#1081). A `self` *introspection* call inside a method body (`[self class]`, `[self object]`) is an ordinary command substitution and contributes nothing to the outline. A dynamic block word (`self $body`) is skipped rather than guessed at.

Within a `self` / `private` block, a member that a later word in the same block **deletes or renames** (`deletemethod`, `renamemethod`) is dropped rather than listed — real Tcl removes it, so keeping the entry would navigate to a name the interpreter does not have. Which member words retract is registry data (the definer grammar's `retracts_named_members`), so the sibling reference members `export` / `unexport` / `filter`, which name a method without removing it, keep their outline entry. A member retracted in one `oo::define` body and redeclared in a later one is listed again — the retraction applies where the body is walked, not document-wide. *Known gap:* an **unwrapped** `deletemethod` / `renamemethod` (one with no `self` / `private` wrapper, directly in a class-creation or `oo::define` body) is not yet honoured, so its member is still listed.

A BIG-IP `.conf` file (any canonical basename — `bigip.conf`, `bigip_base.conf`, …) gets a different outline shape: a `module → kind → object` tree built from the config stanza tree rather than the Tcl scope walk. Nameless global singletons (`auth password-policy`, `net self-allow`, …) fall back to their kind label so no outline entry is ever empty. Both the Python and the native Rust servers serve this outline.

## File-path anchors

- `server/features/document_symbols.py`
- `server/features/_bigip_symbols.py` — BIG-IP `module → kind → object` outline (Python)
- `rust/tcl-lsp-core/src/bigip.rs` — BIG-IP outline + basename detection (native server)
- `rust/tcl-lsp-core/src/document_symbols.rs` — native outline builder (walks the analyser scope tree)
- `rust/tcl-registry/src/symbol_def.rs` — `SymbolDef` / `DefinedSymbolKind` (registry-driven symbol-definer commands)
- `rust/tcl-registry/src/commands/irules/when.rs` — `when` declares its event name as the handler's outline entry
- `rust/tcl-compiler/src/analyser/handlers.rs` — `handle_defines_symbol` (records test cases, constant-propagated name)
- `rust/tcl-compiler/src/analyser/oo.rs` — `expand_wrapper_block_members` (`self { … }` / `private { … }` block forms), `apply_oo_self`
- `rust/tcl-registry/src/definer.rs` — `MemberSpec::wrapper_or_body` (which members take a bare script block), `MemberSpec::retracts_named_members` (which member words delete the members they name, with the oracle transcript)

## Failure modes

- Symbols missing or mis-nested after parser changes.
- VS Code drops the entire outline when any `DocumentSymbol.name` is empty — BIG-IP nameless singletons must fall back to a kind label (#534).

## Test anchors

- `tests/test_document_symbols.py`
- `tests/lsp_e2e/test_bigip_e2e.py` — BIG-IP outline + diagnostic suppression, both backends
- `rust/tcl-lsp-core/src/document_symbols.rs` — TP/FP/TN/FN unit tests for `tcltest` test cases
- `rust/tcl-lsp-server/tests/e2e/document_symbols.rs` — `tcltest_*` e2e cases (document + workspace symbols)
- `rust/tcl-registry/tests/tcltest_specs.rs::test_command_declares_a_symbol_definer`
- `rust/tcl-registry/tests/registry_commands.rs::when_defines_its_event_as_an_outline_symbol`
- `editors/vscode/src/test/documentSymbols.test.ts` — "lists tcltest test cases as symbols", "lists iRules event handlers as Event symbols"

## Screenshots

- `17-document-symbols` — symbol picker showing proc outline

![symbol picker showing proc outline](../screenshots/17-document-symbols.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
