# Current documentation and comments audit

Scope: tracked repository files, including pre-existing material outside naming
changes. Main was read-only. The handoff patch describes current interfaces and
changes comments only in Rust; it grants no new interpreter capability or oracle
conformance. Reproducible experiments, failed runs, original provider sources,
provenance and legal headers remain untouched.

## Patch

`implementation.patch` adds early guide navigation and current contracts for
Module-issued body IDs, exact method/event joins, real summary/lattice memo
inputs, current-source rebinding, scalar source value queries and APL convention
coverage. It corrects obsolete production-map lookup claims in
`command-resolution.md`, stale outward variable-table guidance and dictionary
metadata limits in `name-resolution.md`, and the namespace-model assertion that
one String helper directly implements every runtime store. Syntax checked
constructed namespace reporting is recorded at the entry-point table.

Removed source-comment history includes retired value-reader API names,
formerly duplicated arity rendering, child interpreter rename implementation
history, historical parameter memo behaviour, BPF future migration narrative,
retired DialectSet/OptionRelation descriptions, migrated BIG-IP property labels,
EDA pack migration narrative and a formerly false-positive test classification.
All Rust non-comment lines and assertions are unchanged.

## Repository-wide candidate pass

The tracked Markdown inventory contains 3,979 existing files. The process/history
pattern pass produces 359 candidate lines in 106 files; its complete output is
`/tmp/name-repo-doc-process-candidates-20261014.txt`. Authored source comment and
script/template candidates produce 106 lines; their complete output is
`/tmp/name-doc-comment-candidates-20261014.txt`. Raw provider source/fixture trees
were excluded from editable-source candidates. Candidate matching is an audit
input, not proof that every document or comment is current.

Reviewed legitimate current uses include compiler phases in `GLOSSARY.md` and
`example-walkthroughs.md`; install-script decision/execution phases; actual F5 XC,
partition and account migration features and user instructions; real worktree
build hazards; callback/runtime phases; Tcl version ABI distinctions; intentional
SpecTcl upgrade and unrenderable-field TODO output. These describe present
semantics, user actions or limitations and should remain.

The file called `value-transfers-migration.md` contains a current command-knowledge
inventory, gates and consumer limitations. A name-only search must not delete it
or break its many linked sections. Its historical filename and link labels can
be renamed independently if the final cleanup requires neutral filenames; that
is not required to preserve the current content.

Oracle proof documents contain scripts, provider input/output, source inspections
and failed stages. Experiment-local phases, repeated captures and questions are
reproducibility evidence; they must not be deleted as product-development prose.
Process words inside original upstream source excerpts remain original bytes.

## Remaining final cleanup obligations

* Remove `docs/work/name-handling` tracking/continuation material and the active
  branch red-push exception in `AGENTS.md` only after Root reaches PR readiness.
  Retain the durable owner contracts and actual oracle evidence.
* `rust/tcl-registry/src/f5/mod.rs:49` contains a development-review migration
  hold and numbered acceptance columns. Rewrite against current supported and
  unmeasured context boundaries when that owner is reviewed; retain the existing
  unobserved appliance limits. No F5 row or capability change is proposed here.
* `rust/bigip-report-gen/frontend/src/pages/report.ts:1` explains `@ts-nocheck`
  through JS migration and a restructure commit. Replacing the historical reason
  alone does not resolve the actual untyped frontend debt. Preserve the checker
  directive until its typing owner can supply a correct current reason or fix.
* The current registry model document still identifies proposed extensions
  (`docs/design/registry/dialect-and-package-registry-redesign.md:3`) and records
  development-era rulings near its introduction. Preserve actual unsupported
  model/vocabulary limits; remaining proposal/issue narrative needs a separate
  current-state pass against its real consumers.
* The 10,295-line name implementer guide now has concise routes for the newly
  integrated architecture. A complete semantic/API validation of every older
  section is not established by this candidate scan.
* Run the required documentation/index and formatting gates after integration.
  This patch had no Cargo build and establishes no test pass.
