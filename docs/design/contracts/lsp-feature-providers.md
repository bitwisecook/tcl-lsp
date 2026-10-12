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

### Written qualifiers and constructed metadata

Rename preserves the original written qualifier using the shared
`written_command_tail` owner. Every colon in a separator run and each original
Unicode byte stays in the source prefix; array-element suffixes keep their
selected whole-reference geometry. Complete source/configuration correspondence,
actual reference selection and edit/hazard guards remain independent.

Constructed Itcl class/member labels use `key_holder_and_tail` before the existing
Itcl grammar and class-member checks. Lexical namespace rename preserves that
constructed holder for workspace collision queries, and forget patterns qualify
their recorded constructed source namespace through the shared owner.
Implementation metadata equality removes exactly one root marker, retaining
literal colon segments in ancestry and cycle observations. These projections do
not recover Native namespace paths or supply command/class identity.

The original Native namespace and command routes precede lexical assistance.
Completion's partial-word predicates do not supply namespace lookup or edit
permission; its retained declaration/candidate selection remains independent.

Reference and minifier compatibility labels remove one optional root marker
through `unroot_rooted_key`, preserving literal-colon components. Positioned
known references terminate before lexical metadata assistance. Minifier call-site
selection excludes indirect heads and deduplicates exact spans; these descriptive
labels do not supply the separate public naming-rewrite contract.

### Selected command namespace metadata

Hover and event diagnostics ask `ProfileRegistry::namespace_for_command_spec`
with the command descriptor already selected by the current source context.
The query checks that row's F5 surface and exact constructed namespace. Hover
keeps explicit command profile requirements first. Event diagnostics retain
matched argument-form selection and the existing namespace-hint then
command-requirement fallback. A same-spelled row on a foreign surface supplies no
protocol profiles.
The original whole-source, configuration, Registry and command-role gates remain
at each consumer.

Package suggestions use `ContextRegistry::namespace_package_ownership` over the
actual context and command store. Every selected row in a namespace must name the
same nonempty `required_package` or `tcllib_package`; an unowned row, conflicting
rows or conflicting fields withhold the hint. Namespace and package names may
differ, and namespace matching preserves case and literal colon segments.
Selected C name inputs expose their already projected namespace only for this
metadata query; Jim inputs retain their flat-key projection. Explicit Logical
inputs use the shared written-name owner. These views supply neither a package
loader nor callable identity.

An action still requires the original complete source/configuration, a selected
literal command-head position, unresolved-command evidence, valid package source
syntax, and absence of an existing or dynamic package provider. Metadata cannot
replace those source and edit guards.

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

Workspace reporting checks the requested command store against the retained
context through `RegistrySnapshot::semantic_key`. The complete structural store
identity does not replace current `ContextRegistry` availability or the original
Logical source and lexer configuration. Matching a profile does not supply that
store currency or Native source advice.


The authored package-index advisory walker maps complete written command heads
through the shared written-name convention, then removes one root from the
resulting constructed Registry key. Literal single colons, relative qualifiers,
trailing empty names and dynamic heads cannot borrow an unrelated stock command.
Selected Registry control/body roles and symbolic version/termination conditions
remain required. This convention supplies no C/Jim lookup, package registration,
loader execution or rewrite permission; original analysis-backed index consumers
retain their separate source, context, descriptor and entry guards.

Lexical type hints compare already constructed FunctionUnit and analyser scope
labels through the shared one-root projection. Literal colon components and
Unicode labels remain intact. Hint emission retains the original complete source,
configuration, analysis-owned Registry and independent declaration/SSA domain.


Method receiver projections require the shared whole-variable word under the
actual retained lexer configuration. Text after a braced closer, additional
substitutions and malformed names cannot borrow the variable's class candidates.
Scalar completion handles exclude independently scanned or combined array
indexes and preserve complete braced Unicode names. Partial editor variable
triggers retain their own completion purpose. Source syntax alone supplies no
receiver identity, method navigation, Native dispatch or rewrite permission;
original method and current source/context owners remain independent.


Itcl class-member assistance selects its definition-body family from the actual
analysis-owned Registry. Missing input, unavailable context or an absent grammar
cannot be replaced by a same-profile catalogue and cannot establish a different
class-method family. Explicit standalone constructed metadata accepts a supplied
Registry through the same family/projection worker. This readonly family metadata
supplies no package loading, native class, current method table, lookup or edit
permission. The custom class-member prefix grammar and dispatch precedence retain
their independent source purpose.


Workspace class records retain the descriptor family selected from their actual
analysis input and Registry. Cross-document class-method candidates require a
known family with the appropriate call shape; an unknown family cannot stand in
for a non-Itcl family. Import conflict metadata borrows the indexed document's
same immutable command store under its complete source/configuration context.
Changing that store invalidates settlement even when the dialect label and
reported declarations match. These projections do not establish a native class,
loaded package, installed import or callable method; original source and command
capabilities remain independently guarded.
