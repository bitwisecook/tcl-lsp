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

A member that a later word in the same body **deletes or renames** (`deletemethod`, `renamemethod`) is dropped rather than listed — real Tcl removes it, so keeping the entry would navigate to a name the interpreter does not have. All three spellings are honoured: inside a `self` / `private` block, after a `self` / `private` prefix, and **unwrapped** in a class-creation or `oo::define` body (#1101). Which member words retract — and *which of their arguments* they retract — is registry data (the definer grammar's `MemberRetraction`), so the sibling reference members `export` / `unexport` / `filter`, which name a method without removing it, keep their outline entry, and `renamemethod old new` retracts only `old`. Retraction is **side-scoped**, matching the interpreter: an unwrapped `deletemethod` acts on the instance side and a `self`-scoped one on the class-object side, so a same-named member on the other side keeps its entry (in real Tcl the cross-side spelling is not merely a no-op but a hard error that aborts the whole definition — `method cm does not exist` — so nothing runnable is lost, and that shape now draws a [`W315`](../codes/kcs-diagnostic-w315-class-definition-cannot-run.md) of its own). `renamemethod old new` is a **move**, not a deletion: `old` goes and `new` is listed in its place, carrying the source's parameter list, body and visibility — real Tcl does exactly that (`info class definition ::C new` answers with `old`'s original body, and `[::C new] new` runs it, on tclsh 9.0.4 and 8.6.14 alike). The moved entry's *name* span is anchored at the `renamemethod` call's destination word — the only place the new name is written — so go-to-definition and hover on the renamed member land there, while its body span still points at the original body (#1121). A rename is not a redeclaration, so the family's name-based export default is not re-applied: `renamemethod Priv pub` leaves `pub` unexported and `renamemethod low Up` leaves `Up` exported, matching the interpreter. A dynamic destination (`renamemethod old $new`) still retracts `old` — it really is gone whatever `$new` is — but records no arrival. Because that destination word *is* the moved member's declaration, **renaming** the moved member rewrites it — so the rename safety gate refuses the two new names that would turn the body into one Tcl will not run: the member's own retracted source (`renamemethod old old` → `cannot rename method to itself`) and a name already live on the same side at that point (`method called X already exists`). The mirror is refused too: renaming an ordinary sibling *into* a later `renamemethod`'s destination collides at that site. Both refusals read the same recorded member state the `W315` diagnostic does, so gate and diagnostic cannot disagree, and both honour body order — a destination deleted before the move is free, one declared after it never collides, and a name live only on the *other* side is legal (#1121 review). The gate is **side-local** in both directions: a move on the instance side is no evidence about a class-side member of the same name, so renaming a `self method same` to `old` is allowed even while an instance-side `renamemethod old same` stands (#1178 review). The diagnostic needs no equivalent filter — each move record is captured against its own side's table at its own site, so it never sees the other side's names. A member retracted in one `oo::define` body and redeclared in a later one is listed again — the retraction applies where the body is walked, not document-wide. A retraction whose target this document does **not** declare — the `oo::define ::C { deletemethod m }` half of a two-file program — is instead recorded as a cross-document *tombstone* and applied by workspace method dispatch, so the file that declares `m` stops advertising it once the extension is indexed; this rides the same unordered channel a cross-file `oo::define ::C { method extra … }` addition already used.

A retraction real Tcl would **reject** — a `deletemethod` of a name absent from its own side, or a `renamemethod` onto a name that side already holds — aborts the whole definition, so no class is created at all. The outline still lists the partial class (navigation degrades rather than vanishing, the same judgement a parse error gets) and the offending word draws [`W315`](../codes/kcs-diagnostic-w315-class-definition-cannot-run.md) (#1120). A blocked rename leaves *both* members listed: the source keeps its own name and an existing destination keeps its own body, so no declaration the author wrote is lost to the wreckage. The check reads each side's table state at the point the word runs, exactly as the interpreter does, so `deletemethod b ; renamemethod a b` is silent. It abstains on a dynamic member name, and on a cross-file `oo::define` stub — a record for a class created in another file has no member tables to judge against, and its retraction travels as a tombstone instead.

The same side-scoping governs the `export` / `unexport` words, which change a member's recorded visibility without changing whether it is listed. Which words set visibility is registry data too (the definer grammar's `MemberVisibility`), not a keyword the walker knows. An unwrapped (or `private`-scoped) `export` / `unexport` flips the instance side; a `self`-scoped one flips the class-object side; naming a method that exists only on the *other* side is a silent no-op in Tcl and is recorded as one (#1098). Only the last explicit writer for a name is kept, so `export m ; unexport m` leaves `m` unexported for cross-file dispatch as well as locally. Visibility is not shown in the outline, but it drives method completion and cross-file dispatch, so a class-side `unexport m` must not hide an instance method `m`. Each side keeps its own last-writer record and its own cross-document channel: an instance-side flip travels in `exports`/`unexports` and a class-side one in `class_exports`/`class_unexports`, so a `self unexport m` written in one file now reaches another file's **class-command** dispatch (`::C m`) instead of being lost (#1119). Both channels ride the same `oo::define` stub mechanism as the retraction tombstones, and share its documented caveat: cross-file load order is not knowable from the index, so the workspace takes the union — any record exporting the name keeps it dispatchable.

`filter` is sided the same way. An unwrapped (or `private`-scoped) `filter f` fills the class's instance filter slot (`info class filters`) and intercepts dispatches on *instances*; a `self filter f` fills the class object's own slot (`info object filters`) and intercepts dispatches on the **class command** — `::B cls`, and even `::B new` — leaving instances unfiltered. Verified byte-identical on tclsh 9.0.4 and 8.6.14. Filters are not listed in the outline, but the split matters to rename safety, which must find and rewrite a `self filter X` word as surely as an unwrapped one.

A BIG-IP `.conf` file (any canonical basename — `bigip.conf`, `bigip_base.conf`, …) gets a different outline shape: a `module → kind → object` tree built from the config stanza tree rather than the Tcl scope walk. Nameless global singletons (`auth password-policy`, `net self-allow`, …) fall back to their kind label so no outline entry is ever empty. Both the Python and the native Rust servers serve this outline.

## File-path anchors

- `server/features/document_symbols.py`
- `server/features/_bigip_symbols.py` — BIG-IP `module → kind → object` outline (Python)
- `rust/tcl-lsp-core/src/bigip.rs` — BIG-IP outline + basename detection (native server)
- `rust/tcl-lsp-core/src/document_symbols.rs` — native outline builder (walks the analyser scope tree)
- `rust/tcl-registry/src/symbol_def.rs` — `SymbolDef` / `DefinedSymbolKind` (registry-driven symbol-definer commands)
- `rust/tcl-registry/src/commands/irules/when.rs` — `when` declares its event name as the handler's outline entry
- `rust/tcl-compiler/src/analyser/handlers.rs` — `handle_defines_symbol` (records test cases, constant-propagated name)
- `rust/tcl-compiler/src/analyser/oo.rs` — `expand_wrapper_block_members` (`self { … }` / `private { … }` block forms), `apply_oo_self`, `apply_sided_member_effects` (the one decision path all three spellings take: `retract_named_members` / `apply_visibility_member` / `apply_filter_member`, each scoped by `MemberSide`)
- `rust/tcl-lsp-core/src/workspace_index.rs` — `WorkspaceClass::retracted_members` / `class_exports` / `class_unexports` + `method_dispatch_chain` / `class_method_dispatch_chain` (cross-document retraction tombstones, the per-side effective-export union, and the one shared `dispatch_chain` walk both sides go through)
- `rust/tcl-compiler/src/analyser/handlers.rs` — `emit_w315_definition_cannot_run` (the `via_define` gate that decides whether a retraction is a cross-file tombstone or a definition-aborting error)
- `rust/tcl-registry/src/definer.rs` — `MemberSpec::wrapper_or_body` (which members take a bare script block), `MemberSpec::retraction` / `MemberRetraction` (which member words delete the members they name, and which of their arguments), `MemberSpec::visibility_effect` / `MemberVisibility` (which words export / unexport) — each with its oracle transcript

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
