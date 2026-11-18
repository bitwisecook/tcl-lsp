# Open item: no proof path from a sourced sibling to cross-file member navigation under a pinned release

Opened 18 November 2026 by the LSP features area, on the coordinator's ruling of the same day. Owner: LSP features, with class facts for the class declarations.

This is a missing fact, not a wrong answer: the product refuses, it does not answer wrongly. It ranks behind every wrong answer and is not being built now.

## The case

```tcl
# animal.tcl
oo::class create Animal {
    method speak {} {return animal}
}
# dog.tcl
source animal.tcl; oo::class create Dog {
    superclass Animal
    method speak {} {return [list dog [next]]}
}
set d [Dog new]
$d speak
```

tclsh 8.6.18, 9.0.4 and 9.1.0: `[$d speak]` is `dog animal`, so the override family across the two files is real. Analysed under a pinned release (`tcl8.6` and later), rename of `speak` refuses and the type hierarchy shows no supertype.

## What the index proves today

With `source animal.tcl` in dog.tcl, the server's workspace index resolves the source edge (dog.tcl → animal.tcl at the `source` word) and `method_override_family("::Animal", "speak")` returns both definers.

## Where the scan stops

The per-document scan (`tcl_lsp_core::rename::method_spans_in_document` and the reference, hierarchy and implementation scans built the same way) requires `AnalysisResult::allows_lexical_declaration_advice`. That gate admits only the Logical (lenient `tcl`) input. It is false under every pinned release, sourced or not, by design: "Missing input, Native recipes and hosted source cannot borrow this domain". The gate stays.

## What a Native path would have to prove

- The source edge itself: an unconditional `source` reached in document order, with the path resolved to the indexed file.
- Both class declarations, each by the native analysis of its own document: the base class created before the derived class's `superclass` runs, and nothing between them deleting or redefining it.
- The method tables: the base's `speak`, the override in `Dog`, and their export state at the call, so the dispatch entry and the `next` chain are known.

Navigation (definition, references, hierarchy) may answer from that proof. Edits (rename and refactors) still need the stricter proof the navigation/edits ruling requires: complete coverage of every occurrence the rename must rewrite, and a refusal otherwise.

## Witnesses

Kept in their `source`-proved pinned form, KEEP-ORACLE, open, cause `pinned-sourced-cross-file-proof-path-missing` in `docs/work/test-scrub/ledger/lsp-features.tsv`:

- `tcl-lsp-server` `tests::method_families::method_rename_spans_override_family_across_documents` (rename).
- `tcl-lsp-server` `tests::navigation::hierarchies::original_type_hierarchy_wire_preserves_owner_and_rejects_labels_and_stale_source` (type hierarchy; tclsh: `[info class superclasses Child]` is `::B\uD800`).

The other server rows about sibling files with no load between them move to lenient `tcl`, blocked on class facts publishing workspace classes to the walk under `tcl`.
