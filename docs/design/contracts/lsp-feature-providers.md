# LSP feature providers (non-diagnostics)

What the non-diagnostic language features — hover, completion, rename,
references, symbols, and semantic tokens — may assume, and where their
behaviour lives. Read it when one of them regresses even though parsing and
diagnostics still look correct.

Feature providers live in `rust/tcl-lsp-core`, one module per feature, and are
**pure**: they take a document analysis (plus, where needed, the workspace
index and a registry) and return a result. The transport, the
`ServerCapabilities` advertised at `initialize`, and every client-specific
adaptation live in `rust/tcl-lsp-server`
([project-layout.md](project-layout.md) rule 4).

That split is what lets the same providers back the LSP, the `tcl` CLI verbs,
and the MCP tool surface without three copies of the behaviour.

## Decision rules / contracts

1. **Providers consume shared resolution, they never re-derive it.** A
   provider that needs to know what a call reaches asks the shared resolver,
   not a local name match — see [command-resolution.md](command-resolution.md)
   for the algorithm and its consumers, and
   [cross-file-diagnostics.md](cross-file-diagnostics.md) for the single
   cross-document lookup (`settle_call_against_workspace`) that navigation and
   diagnostics both go through. Two lookups is how go-to-definition and W123
   end up contradicting each other about one name.
2. **Provider responses are deterministic** for an unchanged document plus
   workspace state.
3. A cross-feature change is validated across at least one navigation feature
   and one edit feature. Rename and find-references in particular must agree
   on the same site set — a rename that edits ranges references does not
   report is a corruption, not a cosmetic difference.
4. **Registry data, never per-provider name lists.** Which commands take
   bodies, which arguments name variables or commands, which subcommands
   exist, and which package owns a command are all `CommandSpec` facts. A
   provider matching on a command name is a review defect.
5. **BIG-IP overlays stay context-aware.** Semantic tokens add BIG-IP value
   categories on `bigip*.conf` without regressing Tcl tokenisation, and
   definition resolves cross-object references through the shared BIG-IP
   object model (`rust/tcl-bigip`) rather than a per-provider map. The overlay
   must not activate on ordinary Tcl.
6. **Shared helpers for shared questions.** Proc-reference matching
   (`definition::resolve_called_proc`), iRules enclosing-event context
   (`irules_context.rs`), and package-suggestion ranking
   (`tcl_compiler::text::rank_containment_suggestions`) each have exactly one
   implementation, so definition / references / rename / call hierarchy /
   signature help stay precedence-consistent and code actions rank the same
   way the server command does.

### Runtime feature toggles

Every feature handler is registered unconditionally and checks its enable
flag in the body, returning nothing when disabled, so
`didChangeConfiguration` toggles it without a restart. The only features that
cannot follow the pattern are those whose capability advertisement is fixed
at `initialize`; there are currently no user-configurable restart-required
toggles. Diagnostics are deliberately not a toggle: the server always
advertises the push model and never `diagnosticProvider` (see
[lsp-diagnostics-publication.md](lsp-diagnostics-publication.md)); a setting
must not try to switch the delivery model after initialisation.

## File-path anchors

Providers (`rust/tcl-lsp-core/src/`):

| Feature | Module |
|---|---|
| completion | `completion.rs`, `snippets.rs` |
| hover | `hover.rs` |
| definition / declaration / implementation / type definition | `definition.rs`, `declaration.rs`, `implementation.rs`, `type_definition.rs` |
| references, rename | `references.rs`, `rename.rs`, `rename_safety.rs`, `namespace_rename.rs`, `linked_editing_range.rs` |
| symbols | `document_symbols.rs`, `workspace_symbols.rs`, `namespace_symbol.rs` |
| semantic tokens | `semantic_tokens.rs` |
| call / type hierarchy | `call_hierarchy.rs`, `type_hierarchy.rs`, `caller_frame.rs` |
| folding, selection range, document links | `folding.rs`, `selection_range.rs`, `document_links.rs` |
| signature help, inlay hints, code lens | `signature_help.rs`, `inlay_hints.rs`, `code_lens.rs` |
| code actions, refactors | `code_actions.rs`, `refactor/`, `file_ops.rs` |
| formatting, minification | `formatting/`, `minify.rs` |
| TclOO | `oo_body.rs`, `oo_dispatch.rs` |
| iRules / BIG-IP | `irules_context.rs`, `irules_object_refs.rs`, `bigip.rs` |
| workspace state | `workspace_index.rs`, `source_graph.rs`, `package_resolver.rs` |

Transport and wiring: `rust/tcl-lsp-server/src/lib.rs`.

## Failure modes

- Completion and hover diverge because one provider bypasses the shared
  resolution rules.
- Rename updates a range set that find-references does not report.
- Semantic tokens or symbol providers drift after a parser or scope change.
- A BIG-IP overlay stops activating, or over-highlights unrelated Tcl.
- A provider grows a command-name list that the registry already answers.

## Test anchors

- Per-provider unit tests co-located in each `tcl-lsp-core` module.
- `rust/tcl-lsp-server/tests/e2e/` — the over-the-wire suite, one module per
  feature area.
- `editors/vscode/src/test/` — the rendered outcome in a real client.

## Discoverability

- [Design doc index](../README.md)
- [LSP diagnostics publication](lsp-diagnostics-publication.md)
- [workspace/indexing contracts](workspace-indexing.md)
- [cross-file diagnostics](cross-file-diagnostics.md)
- [shared utility contracts](shared-utility-contracts-rust.md)
