# The compiler, the analyser and SpecTcl 3 as one system

Status: designed 26 November 2026. It joins the compiler crate split (item
90, [overview](crate-split-design.md), [steps](crate-split/)), the registry
split and model ([registry split](../registry/registry-split-design.md),
[dialects and libraries](../registry/dialects-and-libraries.md), items
301–313, 319) and SpecTcl 3 ([305](../registry/model/305-spectcl-3.md),
[313](../registry/model/313-consumer-hooks.md),
[391](../registry/model/391-handle-types.md),
[392](../registry/model/392-error-codes.md), and the consolidated
specification in `registry/spectcl-3/`). Each SpecTcl design assumes its
analysis lives somewhere in "the analyser"; this document says
which crate, which pass, which interface, and how the interprocedural
analyses share one walk. Where it disagrees with a step design, it says so
in § *Corrections to other designs*.

Type names follow the specification's chapter 7 and
[393](../registry/model/393-taint.md) where they decide them (`HandleFacts`,
`LibraryChoice`, `TaintFact`, `ColourSet`, `ImageView`); names first
proposed here are marked *design sketch*.

## The decisions in brief

1. **One facts interface, in the registry core:** `FactView`, a generation
   plus a facet selection. No compiler crate names a surface, the catalogue
   or `tcl-spectcl`.
2. **Every fact is an image, and only `tcl-catalogue` embeds images.** That
   includes the Tcl core's specs, docs and release facts. `tcl-dialect-tcl`
   and the other surface crates hold native hooks only. A fact edit rebuilds
   `tcl-catalogue` and the crates that link it: the binaries, the WASM hosts,
   `xtask` and the `-it` test crates. It rebuilds no generic crate, not even
   its test target (§ 1, *The dependency rule*).
3. **The positioned context is computed in two places** and memoised by
   `tcl-db`. `tcl-binding` computes it inside a body, from that body's entry
   selection. `tcl-unit` computes each body's entry selection top-down, after
   condensation (§ 1).
4. **One summary framework:** domains and their product walk in
   `tcl-analysis`, the solver in `tcl-unit`, the records in `tcl-model`. One
   condensation, and one bottom-up walk with step 7's per-body fixpoint
   inside. Binding stays frozen inside a component until the component
   converges (§ 3). Today's six call-graph fixpoints become its domains.
5. **The VM is reached only through `PackHookHost`,** which exists today.
   Entry points supply a lazy host.
6. **Salsa inputs are packs and the generation they build.** Fine-grained
   invalidation comes from three backdated projections: binding facts, body
   facts and the shared tables. No salsa value holds a generation.
7. **Build time first;** SpecTcl engines wait for their crates.

## 1. Where SpecTcl facts enter the compiler

### What exists today

`tcl-compiler` reads `tcl-registry` directly (`cargo tree -p tcl-compiler
-e normal --depth 1`). Pack hooks cross one trait the registry defines and
`tcl-spectcl` implements: `PackHookHost`
(`rust/tcl-registry/src/pack_hooks.rs:518`, `invoke(slot, call) ->
HookAnswer`), installed process-wide by `install_host`
(`pack_hooks/host_installation.rs:156`) and implemented in
`tcl-spectcl/src/hooks/service.rs:324`. Extensions do the same through
`ExtensionHost` (`extension_host.rs:58`) in a thread-local. So the VM
boundary exists. What is wrong is the global installation, which
[step 14](crate-split/14-caching.md) deletes, and the loader's edge to the
VM, `tcl-spectcl` → `tcl-spec-hooks` → `tcl-engine-tclvm` → `tcl-vm` (§ 6).

### The interface

The registry core owns it: `tcl-model` is compiler vocabulary the VM and
`tcl-spectcl` must not depend on, and 391 and 392 already put `HandleType`,
`ErrorClass`, `Throws` and `CompletionCodes` in the core
([391](../registry/model/391-handle-types.md#rust-parity),
[392](../registry/model/392-error-codes.md#rust-parity)).

```rust
// design sketch: tcl_registry::facts (registry core); names provisional
pub struct Generation { /* surfaces, image bytes (catalogue: 'static; user packs: Arc), grammar profiles, hook table */ }
pub struct GenerationId(u64);                 // item 47: one per install

pub struct FactView<'g> {
    generation: &'g Generation,
    selection: &'g FacetSelection,            // dialect set, library choices, targets (308)
}
impl<'g> FactView<'g> {
    pub fn command(&self, key: CommandKey) -> Option<SpecRef<'g>>;       // facets already selected
    pub fn visible(&self, name: &str) -> Visibility;                     // Must, May (annotated), Absent
    pub fn handle_type(&self, id: HandleTypeId) -> Option<&'g HandleType>;   // 391
    pub fn error_class(&self, id: ErrorClassId) -> Option<&'g ErrorClass>;  // 392
    pub fn grammar(&self) -> &'g GrammarProfile;                            // 370
    pub fn hooks(&self) -> HookDispatch<'g>;  // rungs 3, 4, 5 behind one call
}
pub struct HookDispatch<'g> { table: &'g HookTable, host: Option<&'g dyn PackHookHost> }
```

- **Concrete, not a trait object:** binding and SCCP read it in inner loops,
  and release LTO inlines a borrowed struct across crates.
- **Facets are selected once per view.** A range consumer (W150, completion
  over targets) gets a set selection and a `VersionLine` answer (308).
- **Hooks are one call.** `HookDispatch::answer(family, call)` tries the
  tabulated table (rung 5), the native table (rung 3), then the host
  (rung 4), and otherwise abstains.
- **The host is a field, not a global.** `install_host` survives only in
  entry points until the generation carries the host.
- **Views borrow; salsa values copy.** Every view borrows the generation for
  `'g` (chapter 7, *The model as Rust types*). Catalogue images are the
  `'g = 'static` case. A salsa value never holds a `FactView`, a `SpecRef` or
  an `Arc<Generation>`. It holds `CommandKey`s, interned IDs and copied
  facts, so it compares equal across generations. An old generation is
  dropped once the queries in flight that borrow it finish.
- **Abstention has a polarity per family.** `answer()` returns
  `HookAnswer::Abstained`, and the family's polarity in chapter 5's *Result
  protocols* says what the consumer does with it.

### How images and user packs reach it

Shipped packs, the Tcl core's included, are compiled once per build by the
`tcl-spec-images` generator into the catalogue's untracked `images/`
directory, and embedded by `tcl-catalogue`
([307](../registry/model/307-libraries-as-data.md#images-per-build)). User
and workspace packs are read by
`tcl-spectcl` on `tcl-source` and compiled in memory with no VM
([305](../registry/model/305-spectcl-3.md#the-loader-on-tcl-source)). The
entry point hands both kinds of bytes to the core: one decoder (spec R27),
the tier applied at install (R22). A generation is a value, so queries in
flight never mix generations (item 47's race).

Generic crates' tests read the same image files at run time, through the
unconditional `tcl_registry::testing` module (item 296's option 2). Cargo
does not fingerprint a file a test opens, so an image change recompiles no
test target. A native hook ID the test binary lacks is bound to an
abstaining stub, and the harness prints each stub once.

### How the positioned context selects facets

`FacetSelection` is the part of 308's `PositionedContext` that selection
reads: the dialect set, each `LibraryChoice` and the targets, per transition
interval. 308 puts the query in `tcl-db`, above the version gate
(`tcl-diagnostics`), the `check` atoms and binding's own visibility. So:

- **`tcl-binding` computes it inside a body.** Binding owns the package
  transitions (item 84) and the `Must`/`May` join it already uses for
  `BindingKnowledge` (`rust/tcl-registry/src/model/binding.rs:211`). It
  starts from the body's entry selection, calls the core's
  `resolve_libraries` (302) per interval and returns `ContextIntervals`, a
  sorted list of `(Span, FacetSelection)`.
- **`tcl-db` memoises it** as `positioned_context(db, file, at)`, which looks
  up the interval. The LSP, MCP and the CLI call the query; crates below
  `tcl-db` take `&ContextIntervals` from their caller.

Binding's first round uses the body's entry selection and its second the
intervals, inside step 7's per-body loop; the second is usually a no-op.

**Entry selections, top-down.** 308 makes a procedure's entry state the
join of the states at its known call sites. So a callee's selection depends
on its callers, while summaries flow up from callees. The two cannot share
one walk:

- **Before the call graph exists,** a procedure body's entry selection is
  the top-level selection at the end of its file, with every `Must`
  downgraded to `May`. File top level starts from the environment's
  placements. This is binding's first visit, and every body's rule until
  INT-d's second half lands.
- **After condensation,** `tcl-unit` runs one top-down pass over the
  components in reverse topological order. A body's entry selection is the
  join of the selections at its call sites. A component is iterated until no
  entry selection in it changes.
- **Termination.** Per library, a selection lives in a finite lattice:
  `Absent`, `Must` and `May` over version sets built from the finitely many
  intervals the rows and requires name, and `Unknown` as top. The join is
  monotone, so a component converges within the lattice's height times its
  body count.
- **Feedback into binding.** A changed entry selection can change
  visibility, and so add or remove edges. Each such change spends one of
  § 3's counted re-binding rounds. When the budget is spent, the affected
  sites bind as unknown and the graph is not re-condensed again.
- **Across files.** The pass runs over the unit's call graph, so a `package
  require` edit in `main.tcl` reaches the procedures of `lib.tcl` through
  it. Salsa re-binds only the bodies whose `entry_selection` changed (§ 5).

### The dependency rule

A fact edit, or a hook fix in a `tcl-lib-*` crate, must never rebuild
`tcl-analysis` or its test target. Seven edges can break that, and the rule
closes each:

| Edge | How it would rebuild `tcl-analysis` | Rule |
|---|---|---|
| a normal edge from a generic crate to a surface | every spec edit rebuilds it | forbidden by item 265's gate ([04-dependency-hygiene](../registry/registry-split/04-dependency-hygiene.md#the-gate)); compiler crates depend on the core only |
| a dev edge to `tcl-dialect-tcl`, whose rlib holds the Tcl core's facts | a Tcl core fact edit changes the rlib, so every generic crate's `--lib` test target recompiles | **new:** the Tcl core's facts are an image too ([307](../registry/model/307-libraries-as-data.md#decisions), decision 1); `tcl-dialect-tcl` holds hooks only, so the dev edge costs a recompile only on a hook edit; tests read images at run time through `tcl_registry::testing` |
| Tcl release facts in `tcl-dialect` (`generated_release_facts.rs`, 11,445 lines, 41 direct dependents) | a release-fact edit rebuilds `tcl-dialect` and everything above it, since cargo has no early cut-off ([259](../build-time/259-worktree-cost.md#the-four-edits)); item 300's `include_bytes!` keeps that | **new:** release facts move to the `DIALECTS` section of the Tcl core image; `tcl-dialect` keeps the types. Their 15 readers (`release_facts()` in `tcl-expr`, `tcl-syntax`, `tcl-dialect` and `tcl-registry`) take them from the `GrammarProfile` or the generation (INT-h) |
| a dev edge to `tcl-catalogue` on 265's known list | the catalogue links every `tcl-lib-*`, so a hook fix recompiles that crate's `--lib` tests | the crates item 90 splits out inherit no known-list row; tests that name a surface move to `-it` crates in the same step ([265](../registry/registry-split/04-dependency-hygiene.md#how-generic-crates-choose-their-test-surfaces)) |
| a normal edge to `tcl-spectcl` | a loader edit rebuilds the compiler | forbidden: the loader sits above the core and beside the compiler, never below it ([305](../registry/model/305-spectcl-3.md#the-loader-on-tcl-source)). `tcl-spectcl` takes its `SurfaceSet` from its caller (`registry_for_dialect_from(&SurfaceSet, …)`) and never depends on the catalogue: measured today, `tcl-sslictcl` is under `tcl-bigip` and `tcl-lsp-core` (`cargo tree -i tcl-sslictcl -e normal --offline`), and C16 puts `tcl-spectcl` under `tcl-sslictcl` |
| a `test-support` feature on the core | a feature enabled only from dev-dependencies gives every crate above the core a second unit hash in one worktree (item 296) | an unconditional `tcl_registry::testing` module, item 296's option 2 |
| an edge from the core to the BIG-IP schema (171k lines) | a schema edit rebuilds the core and so everything | the schema crate has its own role, `schema`, whose only dependents are `tcl-bigip`, `tcl-bigip-*` and `xtask` ([265](../registry/registry-split/04-dependency-hygiene.md#crate-roles)) |

The designs missed rows 2, 3, 4, 6 and 7. Item 265's rule 1 made
`tcl-dialect-tcl` every generic crate's default dev-dependency, 307 kept the
Tcl core's facts in Rust, and cargo fingerprints a test target on every
dev-dependency
([265 § Problem](../registry/registry-split/04-dependency-hygiene.md#problem)).
Together, every Tcl core fact edit would have recompiled `tcl-binding` up to
`tcl-diagnostics` as test crates.

**`xtask` is an entry crate.** It links `tcl-catalogue` (today it already
links `tcl-spectcl`, `tcl-compiler`, `tcl-lsp-core` and `tcl-vm`, `cargo tree
-p xtask -e normal,dev --depth 1 --offline`), so every fact edit relinks it,
and every rebuild claim in these designs names it. It does not generate the
images: the generator links no catalogue, so a fresh checkout can build the
images before anything embeds them (307).

It meets 304's rule (no library crate depends on the BIG-IP schema crate,
[304](../registry/model/304-f5-libraries.md#dependencies)): `tcl-catalogue`
must not either, and the profile-to-schema join stays in `tcl-bigip` (360).
A vocabulary change is a core edit and rebuilds everything; § 7 batches the
3.0 vocabulary so it lands once.

## 2. Fact families, crates and passes

Every SpecTcl 3 fact family, the crate and pass that read it, and what they
produce. Crate names are the split's targets; "today" columns are in the
step designs.

| Fact family | Defined in | Read by (crate, pass) | Produces |
|---|---|---|---|
| arity, options, option relations | core `CommandSpec` | `tcl-binding` (`registry_invocation`); `tcl-diagnostics` (arity and option checks); `tcl-ide-*` (signature help, completion) | `ResolvedInvocation`, E002 and option diagnostics |
| roles and invocation resolution (`clause_grammar` → `arg_role_resolver` → `arg_roles`) | core, plus rung 3/4/5 hooks | `tcl-binding`, the one binding walk (item 84) | `InvocationBinding` carried in `tcl-ir`'s `Facts`; the only role source for the minifier, formatter and optimiser (313 decision 1) |
| versioned facets and availability | core, selected by `FactView` | `tcl-binding` (visibility); `tcl-diagnostics` (version gate W135, W136, W144, W150); `tcl-ide-*` (hover `VersionLine`) | per-interval visibility; version diagnostics |
| state transitions (package, namespace, rename, alias) | core descriptors (305 rung 1) | `tcl-binding` | binding timeline; package transitions for the positioned context |
| effects, `optimise -no` | core (`world_effect`, `side_effects`) | `tcl-analysis` (`side_effects` classify, effects domain); `tcl-optimiser` (permissions); `tcl-native-plan` (334) | effect footprints, purity, rewrite permissions |
| results and shapes | core result facets; evaluators in `tcl-values` | `tcl-analysis` (SCCP, `type_infer`, `transfer`) | value lattice facts, return shapes |
| handles (391) | core `HandleType`, producer and consumer rows | `tcl-analysis` (handles domain); `tcl-unit` (solver); `tcl-diagnostics` | `HandleFacts`; wrong-type, use-after-release, leak diagnostics |
| error codes and completion codes (392) | core `ErrorClass`, `Throws`, `CodeSet` | `tcl-analysis` (errors domain, catch and try sites); `tcl-unit` (solver); `tcl-diagnostics`; `tcl-optimiser` (dead `trap` arms) | `CatchFacts`, error sets, completion codes |
| taint (393) | core taint rows; today 15 `CommandSpec` fields | `tcl-analysis` (taint domain, once it reads no built unit); `tcl-unit` (solver); `tcl-diagnostics` (T1xx); `tcl-vm` (run-time taint, from the same rows) | colours, sink findings, taint summaries |
| events, flows, profiles (360, 380) | `tcl-lib-irules` and `tcl-lib-bpf` data, read through the core | `tcl-irules` and `bpf-tcl-ir` as `UnitRule`/`SiteRule` contributions; `tcl-unit` (`connection_scope`); `tcl-irule-test` | event-order checks, CMP findings, test cases |
| grammar profiles (370) | type in `tcl-dialect`; pack profiles in the generation | `tcl-lexer`, `tcl-syntax`, `tcl-expr`, `tcl-source` (CST, segmenter); the writer owner; `tcl-codegen` records the id; `tcl-vm` | tokens, CST, segments; refusals of unsafe edits |
| hooks (rungs 3, 4, 5) | core `HookTable`; native IDs in surface crates; bodies in `BODIES` | every reader of the families they answer, through `HookDispatch` | the family's answer, or `Abstained` read with the family's polarity |
| `check` rules (rung 2) | core rule table | `tcl-diagnostics`, per site, with a `ctx` built from binding, SCCP and the positioned context | pack diagnostics (`PACK/ID`, spec R25) |
| rewrites and proofs (313, 335) | core `RULES` and `PROOFS` | `tcl-optimiser` (rewrite pass); `tcl-ide-format` (aggressive minify, through `tcl-optimiser`) | proved rewrites, gated by the positioned target set |
| layout (313) | core `RULES` | `tcl-ide-format` (formatter) | break choices only |
| scope and frozen names (335) | core role rows | `tcl-ide-format` (minifier) | per-name abstention |
| migrations (`migrate`, 390e) | core | `tcl-diagnostics` (finding plus fix); `tcl spec upgrade` and `find-legacy` in the CLI | modernisation findings and fixes |
| callbacks (390f, `-appends`) | core role rows | `tcl-binding` (callback edges); `tcl-unit` (callback edges in the call graph) | call edges into callback bodies, so handles and taint flow into them |
| embedded ranges (390d) | core role rows | `tcl-binding` (per-site range); `tcl-ide-tokens` (injection metadata) | ranges and editor injection data; never a parser |
| codegen facts (334) | core: purity, effect classes, `alias_of`, `semantic_operation`, `runtime_backing` | `tcl-native-plan`, `tcl-compile`; `tcl-vm` admission (`PackFactStamp`) | specialised sites, admission checks |
| eBPF recipes (380) | `tcl-lib-bpf` and `tcl-lib-linux-ebpf` data | `bpf-tcl-ir` (closed `EbpfOp` catalogue); `bpf-tcl-codegen` (`KernelAbi`) | BPF IR and objects |
| SslicTcl document grammars (365) | `tcl-lib-sslictcl` | `tcl-spectcl` declaration reader on `tcl-source`; `tcl-sslictcl` model | `SslicModel` |
| `test` rows | source only | xtask conformance runners | conformance rows; dropped from the image |

### Facts no crate owns cleanly

- **The positioned context,** placed above its readers: § 1 and C5.
- **Certainty, taint colours and completion codes,** each with two to four
  types today: C2, C3 and C4 in § 8.
- **Fold evaluators,** split by 117b: C12.
- **`connection_scope`,** iRules analysis in `tcl-unit` (step 13). Under
  391e the connection lifetime is a handle lifetime, so the handles domain
  takes it, and the rest becomes a `UnitRule` in `tcl-irules`.

## 3. One interprocedural framework

### The engines today


| Engine | Summary record | Call-graph walk |
|---|---|---|
| purity and effects | `ProcSummary` (`interprocedural.rs:264`) | transitive calls per procedure (`interprocedural/summary_fixpoints.rs:91`), then round-robin `loop` in `fixpoint_pure` (`:110`, `:119`) and `while changed` in `fixpoint_effects` (`:179`, `:190`) |
| value transfer | `TransferSummary` (`interprocedural/transfer.rs:112`) | Tarjan components (`transfer/summarisation.rs:430`), callees before callers (`:41`) |
| return values | — | `summarise_returns` (`interprocedural/return_summaries.rs:331`) |
| taint | `ProcTaintSummary` (`taint_interproc.rs:185`) | a dirty-set worklist (`taint_interproc/summary_fixpoint.rs:19`–`48`), and again per procedure in `tcl-lsp-db` as `proc_summary_cascade` (`check_solve.rs:115`, `lru = 512`) |
| object types | `ObjectHandleFacts` (`object_types.rs:248`) | `loop` over the whole unit's flow edges until no set grows (`object_types/object_flow.rs:97`) |
| escape | `ProcEscapeSummary` (`var_escape/types.rs:273`) | `while changed` (`var_escape/interprocedural.rs:273`) |

Six walks over one call graph, three convergence schemes, two copies of the
taint cascade. 391, 392 and 393 would add three more of the same shape.

### The design

One framework in three layers, following step 10's tiers:

- **the records** in `tcl-model`, so the LSP, MCP and the explorer print
  them without naming the engines;
- **the `SummaryDomain` trait, each domain's transfers and the product walk**
  in `tcl-analysis` (`tcl_analysis::summary`), beside step 10's
  `ProcedureSummaries` trait: a body's walk reads its callees only through
  summaries, never a built unit, so it is a per-function pass;
- **the solver** (condensation, component loop) in `tcl-unit`, the fixpoint
  driver of step 7.

```rust
// design sketch: proposed here; record names provisional (spec chapter 7)
// tcl-model
pub struct ProcSummary {                       // one record per body, every domain in it
    pub body: BodyKey,
    pub base: BaseSummary,                     // today's ProcSummary fields: arity, calls, purity, completes
    pub transfer: Option<TransferSummary>,
    pub handles: HandleSummary,                // 391: parameter needs, returned type, releases
    pub errors: ErrorSummary,                  // 392: raises, completion codes
    pub taint: ProcTaintSummary,               // 393's gen/kill shape
    pub escape: EscapeSummary,
}
// tcl-analysis
pub trait SummaryDomain {
    type Fact: Lattice;                        // per SSA value or site, in one body
    type Summary: Lattice;                     // per body
    fn unknown_callee(&self, sig: &Signature) -> Self::Summary;     // top: what an unknown command does
    fn transfer(&self, cx: &FlowCx<'_>, at: StmtId, state: &mut Self::Fact, others: &ProductView<'_>);
    fn apply_call(&self, callee: &Self::Summary, call: &CallSite<'_>, state: &mut Self::Fact);
    fn summarise(&self, exits: &ExitStates<'_, Self::Fact>) -> Self::Summary;
}
```

- **The product.** A body's flow state is every domain's `Fact`. One
  forward walk over the SSA CFG calls each domain's `transfer` per statement,
  in a fixed order (errors and completion codes, handles, taint, escape);
  each reads the earlier ones through `ProductView`. SCCP stays its own
  sparse solver, runs first, and fixes the executable blocks.
- **The reduction is the point.** After `catch {open $p a} ch`, `ch` is a
  channel only where `catch` returned 0: the handles domain needs the errors
  domain's codes and SCCP's branch. Separate engines need a second pass.
- **One condensation** of the call graph from binding's resolved invocations
  (item 84) plus callback edges (390f), by one Tarjan pass.
- **One bottom-up walk.** Components are visited callees first. Inside a
  component the summaries are iterated until none changes, with binding
  frozen (below); past a per-domain height bound a summary widens to
  `unknown_callee`. The unit of work and of caching is the component
  (`component_summaries`, § 5), not one query per body, so the solver needs
  no salsa cycle recovery: a component's query calls only lower components'
  queries.
- **Scoped propagation.** Facts are keyed `(owner, variable)` as
  `ObjectHandleFacts::by_scope` is (`object_types.rs:275`). Only summary
  edges cross bodies: parameters, returns, callback arguments, a class's
  instance variables.
- **Certainty and proven reads.** As with `proven_reads`
  (`object_types.rs:277`), an edit, rename, fold or error needs a `Must` fact
  at an exact SSA read; a `May` fact may only label, annotate or warn.

```mermaid
flowchart TB
  binding["tcl-binding: resolved invocations and callback edges"] --> callgraph["one call graph"]
  callgraph --> scc["one Tarjan condensation"]
  scc --> walk["bottom-up walk over components"]
  subgraph perBody["per body, inside the walk"]
    fix["binding and SCCP fixpoint, step 7"] --> flow["one forward walk over SSA"]
    flow --> errs["errors and completion codes"]
    flow --> hnd["handles"]
    flow --> tnt["taint"]
    flow --> esc["escape"]
  end
  walk --> fix
  errs --> prod["ProcSummary: the product"]
  hnd --> prod
  tnt --> prod
  esc --> prod
  prod --> walk
  facts["FactView: rows for every domain"] --> flow
  prod --> consumers["diagnostics, optimiser, native plan, LSP"]
```

### Against the binding fixpoint and item 84

Step 7's loop is per body (binding, lowering, SSA, SCCP, binding again,
[07](crate-split/07-binding-fixpoint.md#the-choice-inversion)); the summary
walk is across bodies. They nest: a body's loop runs inside its component's
visit, with its callees' summaries reaching SCCP through step 10's
`ProcedureSummaries` trait. So O103's pure-proc fold can feed binding, as
spike 94's `e01-computed-head` does within one body.

A proved operand can bind an open call, and a widened summary can take a
proved operand away, so edges move both ways. Binding is therefore frozen
while a component iterates:

1. **Binding reads converged summaries only.** A body's step 7 loop binds
   against a callee's summary only once the callee's component has
   converged. A callee in the same component reads as `unknown_callee` for
   binding, while the summary iteration itself reads the current estimate.
   With binding fixed, every domain's transfer is monotone, so the component
   converges within its height bound.
2. **Re-binding after convergence.** The component's final summaries are
   then given to binding once. If no binding changes, the component is done.
   If one changes (an edge added or removed), the affected bodies re-bind,
   the call graph is re-condensed where the edge crosses components, and
   step 1 runs again for the components that changed.
3. **A counted budget.** A component gets at most three such rounds; the
   count is per component and shown by the explorer. When the budget is
   spent, every site whose binding changed in the last round binds as
   unknown, its callee summary is `unknown_callee`, and the component
   iterates once more with binding frozen. That result is sound, because
   `unknown_callee` is top in every domain.

Callers above a component see only its final summaries. Item 84's one walk
per body supplies the call, callback and package-transition edges.

**Example.** In `proc g {n} { if {$n>0} {return x[g [incr n -1]]}; set c [g
0]; $c hi; return puts }`, `g`'s first estimate returns `puts`, so SCCP could
bind `$c hi` to `puts`. Step 1 forbids it: `g` is its own component, so
binding reads `unknown_callee` and `$c hi` stays an open call. Once the
component converges, `g` returns `puts` or `x…`, which proves no command
name, so step 2 binds nothing and the component is done.

### What it costs

- **Time.** One O(V + E) condensation; one walk per body visit for every
  domain. The round-robin loops and the whole-unit object-flow rescan per
  round (`object_flow.rs:97`) go. A new domain costs its transfers, not a
  walk.
- **Memory.** One `ProcSummary` per body; site facts only for the body asked
  about. Of `ObjectHandleFacts`' module-wide maps only `proven_reads` stays.
- **Budgets,** INT-e's gate, on release builds against item 106's classes:
  - **Edit.** A one-line edit inside a procedure body, then a point query in
    a caller (hover, or a diagnostic at the cursor), within item 106's 100 ms
    after an edit. The summary wave counts against it. A wave that runs past
    the budget answers the query from the components already recomputed and
    finishes the rest under salsa cancellation (item 105).
  - **Cold.** Summaries for a whole unit at item 106's collection rate,
    300 ms per 10,000 lines, parse and binding excluded. tcllib 2.0 in
    `tmp/` is 883 files, 459,905 lines and 8,892 `proc` lines (counted 26
    November 2026), so 14 s. Today's cold scan is 58 s for 10,513 lines (item
    103), so this is a target to meet, not a measurement.
  - **Memory.** Step 14's resident-memory probe, before and after. At 1–3 KB
    per `ProcSummary`, tcllib's summaries are 9–27 MB.
  - Per-domain release latency on the item 46 corpus is recorded beside
    them.

## 4. Pass order and data flow

Each pass, with the facts and hook rungs it consults:

| Pass | Crate | Facts consulted | Hook rungs |
|---|---|---|---|
| environment and grammar profile | `tcl-dialect`, core | detection rows, pack `dialect` blocks | — |
| lexing | `tcl-lexer`, `tcl-expr`, `tcl-syntax` | `GrammarProfile` | — |
| CST and segmentation | `tcl-source` | `GrammarProfile` (the F5 `else` merge axis) | — |
| binding | `tcl-binding` | visibility, roles, clause grammars, transitions, callbacks, embedded ranges | 3, 4, 5 (roles, prefix, timing) |
| positioned context | `tcl-binding` | library and dialect rows, placements | — |
| lowering | `tcl-lowering` | lowering hook IDs (closed native set) | 3 (core only) |
| IR, CFG, SSA | `tcl-ir`, `tcl-lowering` | — | — |
| SCCP and values | `tcl-analysis`, `tcl-values` | results, shapes, folds | 3, 4 (fold) |
| entry selections | `tcl-unit`, top-down after condensation | library and dialect rows, call sites | — |
| summary walk: errors, handles, taint, escape | `tcl-analysis` domains, `tcl-unit` solver | `throws`, handle rows, taint rows (the sink gate is a rung 2 `SinkWhen` predicate, not a hook), effects | 3, 4 (effects) |
| diagnostics | `tcl-diagnostics`, `tcl-irules`, `bpf-tcl-ir` | `check` rules, version facets, migrations, events | 2, 3, 4 (diagnostic family) |
| optimiser | `tcl-optimiser` | effects, `optimise -no`, rewrites, proofs | 3, 4 (rewrite) |
| minifier, formatter | `tcl-ide-format` | roles (through `ResolvedInvocation`), scope, frozen, layout | 3, 4 (layout); rewrites through `tcl-optimiser` |
| native plan and codegen | `tcl-native-plan`, `tcl-compile`, `tcl-codegen` | codegen facts only (334) | none (spec R19) |
| tbcx | `tcl-codegen`, item 311 | the unit's `GrammarProfileId` | — |

```mermaid
flowchart LR
  text["source text"] --> env["environment and GrammarProfile"]
  env --> lex["lexer"]
  lex --> cst["CST and segmenter"]
  cst --> bind["binding"]
  bind --> ctxq["positioned context"]
  ctxq --> bind
  bind --> lower["lowering, CFG, SSA"]
  lower --> sccp["SCCP"]
  sccp --> bind
  sccp --> walk["summary walk"]
  walk --> diag["diagnostics"]
  walk --> opt["optimiser"]
  diag --> lspOut["LSP, MCP, CLI"]
  opt --> fmt["minifier and formatter"]
  opt --> plan["native plan"]
  plan --> cg["codegen and tbcx"]
  hookSrc["HookDispatch: rungs 3, 4, 5"] --> bind
  hookSrc --> sccp
  hookSrc --> walk
  hookSrc --> opt
  checks["check rules, rung 2"] --> diag
  cgFacts["codegen facts only"] --> plan
```

### Where the VM is needed

Only rung 4, a Tcl hook body with no tabulated or native answer, needs an
interpreter. The core defines the boundary, `tcl-spec-hooks` implements it,
and the entry point wires them:

```rust
// design sketch: the existing trait, held instead of installed
// tcl-registry core (exists: pack_hooks.rs:518)
pub trait PackHookHost { fn invoke(&self, slot: HookSlot, call: &HookCall<'_>) -> HookAnswer; /* .. */ }
// tcl-spec-hooks (exists: tclvm_host, lib.rs:108), wrapped lazily
pub struct LazyHost { cell: OnceCell<HookHost<TclVmEngine>>, config: HostConfig }
impl PackHookHost for LazyHost { /* builds the VM on the first invoke, then delegates */ }
```

```mermaid
sequenceDiagram
  participant A as tcl-analysis fold
  participant D as HookDispatch in the core
  participant T as image TABLES
  participant H as LazyHost in the entry point
  participant V as tcl-vm
  A->>D: answer(fold, call)
  D->>T: tabulated answer for this shape?
  T-->>D: miss
  D->>H: invoke(slot, call)
  H->>V: first call builds the VM and loads the TbcxUnit
  V-->>H: result or error within fuel
  H-->>D: HookAnswer or abstention
  D-->>A: value, or Abstained, read with the family polarity
```

- The compiler and `tcl-ide-*` crates reach the host only through
  `HookDispatch`, and none depends on `tcl-vm`, `tcl-spec-hooks` or
  `tcl-spectcl`.
- An entry point without Tcl hooks links no VM; its rung 4 calls abstain.
  Spec Studio enables them for the author tier (spec trust table).
- **Fuel only.** An answer analysis consumes runs under the body's fuel, a
  command count, never a wall-clock cap. So the same inputs give the same
  answer on a loaded machine, and step 14's fresh-against-warm gate holds. A
  watchdog may still stop a runaway call; that quarantines the hook and moves
  `HookHostEpoch`, which invalidates every answer the hook gave.
- **A body that reads `values` runs once per site.** Such a roles, prefix or
  timing body feeds binding, which feeds SCCP, which feeds `values`. It is
  called in step 7's first round only, and later rounds reuse that answer,
  so a non-monotone body cannot make the loop oscillate (chapter 5, rung 4).
- The VM reads the registry core (admission, run-time taint), never the
  analysis crates. 393 tags a hook body's input words with the analyser's
  static `TaintFact`s; those are core types carried in `HookCall`, so the
  tagging adds no edge.

## 5. Incrementality and caching

The rule stays [step 14's](crate-split/14-caching.md#the-rule-per-crate):
lower crates are pure, `tcl-db` holds every cache.

### Inputs

`tcl-lsp-db`'s inputs today are `EvaluatorEpoch` (`lib.rs:556`),
`OverlayEpoch` (`:584`), `SourceFile` (`:699`), `AnalyserConfig` (`:810`),
`Project` (`:919`) and `RetainedModules` (`memoised_units.rs:239`, which
step 14 deletes). The integration adds:

| Input | Holds | Set by |
|---|---|---|
| `CompiledSurfaces` (singleton) | the catalogue's image bytes and hook tables | the entry point, once |
| `UserPack` (one per pack) | source hash, tier, compiled image `Arc` | the pack watcher; replaces `OverlayEpoch`'s role |
| `HookHostEpoch` | today's `EvaluatorEpoch`, renamed: moves on quarantine or reload | the hook host |

Everything else is derived:

- `generation(db)`: item 47's registry generation. Only the projections
  below read it, so no other query depends on the whole generation;
- `grammar_profile(db, file) -> GrammarProfileId`, where the id is a content
  hash of the profile, so a pack edit that leaves a profile equal re-parses
  nothing; the CST is keyed on text and profile id;
- `visible_names(db, env)`: the names an environment can bind, no facts;
- `binding_fact_slice(db, body)`: for each command the body resolved, what
  binding reads: roles, clause grammars, transitions, callbacks, embedded
  ranges, and hook IDs with their polarity;
- `body_fact_slice(db, body)`: for each command the body resolved, what the
  summary domains read: `throws`, handle rows, taint rows, effects, results;
- `fact_tables(db)`: the rows no command owns: colours, sinks, origins,
  handle types, error classes and the `default` throws rows. Every summary
  depends on it;
- `hook_table(db)`: the projection of the generation's hook table that
  `HookDispatch` reads, compared by content;
- `entry_selection(db, body)`, over `tcl-unit`'s top-down pass, and
  `positioned_context(db, file, at)`, over `tcl-binding`'s intervals;
- `call_graph(db, unit)` with its condensation, one value per unit;
- `component_summaries(db, component) -> Arc<[ProcSummary]>`, keyed by the
  component's sorted `BodyKey`s, and `body_summary(db, body)` as a
  projection of it. They replace `proc_summary_cascade` and the other
  per-engine queries.

Every one of these holds IDs and copied facts, never a view into the
generation.

### A user-pack edit

A user edits `redis.tclspec` and changes the `throws` row of `redis::get`:

```mermaid
flowchart TB
  edit["UserPack input: new hash"] --> gen["generation: rebuilt"]
  gen --> names["visible_names: equal, backdated"]
  gen --> bslice["binding_fact_slice of every body: equal, backdated"]
  gen --> tables["fact_tables: equal, backdated"]
  gen --> slice1["body_fact_slice of a body calling redis::get: changed"]
  gen --> slice2["body_fact_slice of every other body: equal, backdated"]
  names --> bindq["binding per file: not re-run"]
  bslice --> bindq
  slice1 --> sum1["component_summaries of that body's component: re-run"]
  slice2 --> sum2["other components: not re-run"]
  tables --> sum2
  sum1 --> callerQ{"summary changed?"}
  callerQ -- "yes" --> callers["callers' components re-run"]
  callerQ -- "no" --> stop["backdated: callers untouched"]
  callers --> diagQ["diagnostics of affected files"]
```

Binding reads `visible_names` and `binding_fact_slice`. An edit that adds or
removes no name, and changes no role, grammar, transition, callback or
embedded range, re-binds nothing. A role edit (`redis::get`'s key word to
`-role Body`) changes the binding slice of each body that calls it, and
re-binds those bodies. A table edit (a new `colour`, a widened `-safe`, a
`handle` row, an `errorclass` window) changes `fact_tables` and re-runs every
summary, which is what such an edit means. Slices are recomputed (a lookup
per resolved command) and backdate when equal. A component whose summaries
did not change stops the wave, as `SummaryDepsKey` does for taint today
(`check_solve.rs:95`).

### Not cached, on purpose

- positioned contexts per offset (only per interval);
- a body's site facts (recomputed when asked);
- per-domain summaries (one product query);
- rung 4 answers: not salsa values; tabulation at image build is the cache,
  and the host's existing slot cache stays entry-point state;
- `FactView` (a borrow) and old generations (dropped, no LRU).

## 6. Interfaces per consumer

### Today

Measured 26 November 2026 with `cargo tree -p <crate> -e normal --depth 1
--offline` (workspace crates shown). Cone: unique packages in `cargo tree -e
normal,build --prefix none`. VM: `cargo tree -e normal -i tcl-vm`.

| Consumer | Direct workspace edges today | Cone | `tcl-vm` in cone |
|---|---|---|---|
| `tcl-lsp-server` | `f5-xc`, `tcl-bigip`, `tcl-bigip-query`, `tcl-compiler`, `tcl-core-types`, `tcl-diagram`, `tcl-dialect`, `tcl-explorer`, `tcl-lexer`, `tcl-lsp-core`, `tcl-lsp-db`, `tcl-registry`, `tcl-spectcl`, `tcl-syntax`, `tcl-version` | 202 | yes, through `tcl-spectcl` → `tcl-spec-hooks` |
| `tcl-mcp` | `f5-cli`, `f5-xc`, `tcl-bigip`, `tcl-cli`, `tcl-cli-support`, `tcl-codegen`, `tcl-compiler`, `tcl-diagram`, `tcl-dialect`, `tcl-irules`, `tcl-lexer`, `tcl-lsp-core`, `tcl-registry`, `tcl-spec-studio`, `tcl-spectcl`, `tcl-syntax`, `tcl-version` | 341 | yes |
| `tcl-cli` | `tcl-cli-support`, `tcl-codegen`, `tcl-compiler`, `tcl-diagram`, `tcl-dialect`, `tcl-explorer`, `tcl-lexer`, `tcl-lsp-core`, `tcl-pkg`, `tcl-registry`, `tcl-sandbox`, `tcl-spec-studio`, `tcl-spectcl`, `tcl-syntax`, `tcl-version` | 227 | yes |
| `tcl-lsp-core` (formatter, minifier) | `tcl-bigip`, `tcl-cmd-core`, `tcl-compiler`, `tcl-core-types`, `tcl-dialect`, `tcl-irules`, `tcl-lexer`, `tcl-platform`, `tcl-regex`, `tcl-registry`, `tcl-sslictcl`, `tcl-syntax` | 134 | no |
| `tcl-compiler` (optimiser) | `tcl-bytecode`, `tcl-cmd-core`, `tcl-core-types`, `tcl-dialect`, `tcl-expr`, `tcl-lexer`, `tcl-regex`, `tcl-registry`, `tcl-runtime-api`, `tcl-syntax` | 64 | no |
| `tcl-codegen` | `tcl-bytecode`, `tcl-cmd-core`, `tcl-compiler`, `tcl-core-types`, `tcl-dialect`, `tcl-lexer`, `tcl-registry`, `tcl-runtime-api`, `tcl-syntax` | — | no |
| `tcl-vm` | `tcl-bytecode`, `tcl-cmd-core`, `tcl-core-types`, `tcl-dialect`, `tcl-host-native`, `tcl-lexer`, `tcl-platform`, `tcl-regex`, `tcl-registry`, `tcl-runtime-api`, `tcl-syntax`, `tcl-value-syntax` | — | itself |
| `tcl-irule-test` | `tcl-bigip`, `tcl-bytecode`, `tcl-cmd-core`, `tcl-codegen`, `tcl-compiler`, `tcl-dialect`, `tcl-irules`, `tcl-lexer`, `tcl-registry`, `tcl-runtime-api`, `tcl-syntax`, `tcl-vm` | — | yes, directly |
| `tcl-explorer` | `tcl-codegen`, `tcl-compiler`, `tcl-dialect`, `tcl-lexer`, `tcl-registry`, `tcl-spectcl`, `tcl-syntax` | 148 | yes |
| `tcl-spec-studio` | `tcl-compiler`, `tcl-dialect`, `tcl-lexer`, `tcl-lsp-core`, `tcl-registry`, `tcl-spectcl`, `tcl-syntax` | 148 | yes |
| `bpf-tcl-ir` | `tcl-compiler`, `tcl-lexer`, `tcl-registry`, `tcl-syntax` | 65 | no |
| `tcl-sslictcl` | `tcl-compiler`, `tcl-core-types`, `tcl-dialect`, `tcl-lexer`, `tcl-syntax` | 126 | no |

`runtime/rust` is its own workspace (`Cargo.toml:142`–`148`); it depends on
`tcl-registry` and the foundation crates (`runtime/rust/Cargo.toml:74`–`102`).

### Target

| Consumer | Depends on (normal) | Never on |
|---|---|---|
| LSP server | `tcl-db`, `tcl-ide-*`, `tcl-catalogue`, `tcl-spectcl` (loader); `tcl-spec-hooks` behind its `tcl-hooks` feature | — |
| MCP, CLI | `tcl-db` for analysis; `tcl-compile` for `dis`, `compwasm`; `tcl-catalogue`; `tcl-spec-hooks` by feature | `tcl-lsp-server` |
| formatter, minifier (`tcl-ide-format`) | `tcl-binding`, `tcl-model`, `tcl-optimiser` (rewrites), the core | `tcl-db`, `tcl-vm` |
| optimiser (`tcl-optimiser`) | `tcl-unit` and below, the core | surfaces, `tcl-spectcl` |
| codegen (`tcl-compile` over `tcl-codegen`) | `tcl-native-plan`, `tcl-unit`, the core | `tcl-diagnostics`, `tcl-optimiser` (item 292) |
| VM, runtime | the core, `tcl-native-model`, foundation crates | every compiler crate |
| iRules test framework | `tcl-compile`, `tcl-vm`, `tcl-irules`, `tcl-lib-irules` facts through the catalogue | `tcl-lsp-*` |
| explorer | the split crates its views read, `tcl-compile`; no `tcl-spectcl` ([294](../workspace/294-explorer.md)) | `tcl-vm` |
| Spec Studio | `tcl-spectcl`, `tcl-ide-format`, `tcl-ide-names`; `tcl-spec-hooks` for the author tier | `tcl-lsp-core` |
| `bpf-tcl-ir` | `tcl-ir`, `tcl-lowering`, `tcl_native_plan::target_contract`, the core | `tcl-analysis` (see below) |
| `tcl-sslictcl` | `tcl-source`, `tcl-spectcl`'s declaration reader (365) | `tcl-compiler` crates |

```mermaid
flowchart TB
  lsp["tcl-lsp-server"] --> db["tcl-db"]
  mcpCli["tcl-mcp, tcl-cli"] --> db
  mcpCli --> compile["tcl-compile"]
  lsp --> ide["tcl-ide-*"]
  ide --> db
  fmt["tcl-ide-format"] --> optim["tcl-optimiser"]
  fmt --> binding["tcl-binding"]
  db --> diag["tcl-diagnostics"]
  db --> optim
  compile --> nplan["tcl-native-plan"]
  compile --> codegen["tcl-codegen"]
  irt["tcl-irule-test"] --> compile
  irt --> vm["tcl-vm"]
  diag --> unit["tcl-unit"]
  optim --> unit
  nplan --> unit
  unit --> analysis["tcl-analysis"]
  analysis --> lowering["tcl-lowering"]
  bpf["bpf-tcl-ir"] --> lowering
  lowering --> binding
  binding --> core["tcl-registry core"]
  vm --> core
  ssl["tcl-sslictcl"] --> source["tcl-source"]
  binding --> source
```

`bpf-tcl-ir` stops at `tcl-lowering`: only `semantic_bridge.rs` names the
analysis modules, and only its tests call it
([380 § Problem](../registry/model/380-spectcl-ebpf.md#problem)).

### The target crate graph with the registry and SpecTcl crates

```mermaid
flowchart TB
  entry["entry crates: tcl-lsp-server, tcl-mcp, tcl, tclvm, xtask"] --> catalogue["tcl-catalogue: every image"]
  entry --> db["tcl-db"]
  entry --> hooksRun["tcl-spec-hooks, feature-gated"]
  entry --> loader["tcl-spectcl: reader3, compile_image"]
  gen["tcl-spec-images: the image generator"] --> loader
  gen --> hooksRun
  catalogue --> surfaces["tcl-dialect-tcl, -jim, -f5, tcl-lib-* with hooks: native hooks only"]
  hooksRun --> vm["tcl-vm"]
  db --> diag["tcl-diagnostics, tcl-optimiser"]
  diag --> unit["tcl-unit: summary solver"]
  unit --> analysis["tcl-analysis: summary domains"]
  analysis --> lowering["tcl-lowering"]
  lowering --> binding["tcl-binding: positioned context"]
  binding --> ir["tcl-ir"]
  ir --> model["tcl-model: summary records"]
  model --> values["tcl-values"]
  values --> names["tcl-names"]
  names --> source["tcl-source"]
  loader --> source
  loader --> core["tcl-registry core: FactView, HookDispatch, PackHookHost"]
  model --> core
  surfaces --> core
  vm --> core
  hooksRun --> core
  core --> dialect["tcl-dialect: GrammarProfile"]
  source --> dialect
```

## 7. Sequencing

The owner puts build time first, then the model, then SpecTcl 3 and the
consumers ([plan](../../work/name-handling/plan.md), working order). The
waves:

| Wave | Items | Why here |
|---|---|---|
| 1. Green build-time steps | 259, 246 (266–268), 269–272, 247, 273 | measured baseline; test cascades cut; module map |
| 2. Registry inversion and core | 262, 263, 117a, 118; then 264 and 265 | the core becomes the only thing generic crates name |
| 3. Vocabulary crates | 251 and 281 (`tcl-source`), 248, 280, 249 and 275–277 (`tcl-model`), 252 | `tcl-source` unblocks 305's reader; `tcl-model` takes the summary records |
| 4. 3.0 vocabulary, once | 301–304, then 305's model types with 400–405 (parity), 410 (dependencies), 417 (`GrammarProfile`), 390a–390f, 391a, 392a, 465–468 (taint rows); INT-a, INT-c, INT-l | every new fact family is a core edit that rebuilds everything; landing them together pays that once |
| 5. Binding and the passes | 278, 250, 279, 253, 282, 254, 255, 117b | the crates the engines land in |
| 6. Top of the stack | 256, 283, 257, 284, 285, 258, 286, 287 | `tcl-unit` and `tcl-db` exist |
| 7. SpecTcl loader and images | 288, 305's reader, 306 with 406, 407, 307 (with INT-b), 416, 318, 312, 311 | images need the catalogue; the hook VM needs 312 and 311 |
| 8. Integration | INT-d (308, with item 84's transitions), INT-e, INT-f (391b, 392b–c, 469–471), INT-g, INT-h with 418–420, INT-i | the framework needs `tcl-unit`; the salsa slices need `tcl-db` |
| 9. Consumers | 313, 413, 414; 319–330 in 319's waves; 437–441 (source tools), 442–451 (F5), 452–457 (SslicTcl), 458–464 (eBPF), 472–474; INT-j, INT-k | each `tcl-ide-*` crate is born at pin zero |
| 10. Caching | step 14, then 101, 103 after 83 and 84 | last, measured |
| deferred | 475–479 (run-time taint), with the engine items | VM work stays deferred except 311 and 312 |

What waits: 305's reader for `tcl-source` (251) and the source store (288);
306 and 307 for `tcl-catalogue` (264); 308 for 302, 303, `tcl-binding`
(253), item 84 and `tcl-db` (286); 391b, 392b–c and 469–471 for INT-e,
so `tcl-unit` (258), while their rows need only the core; 313's rewrites for
`tcl-optimiser` (284), layout for `tcl-ide-format` (289), `TbcxUnit` bodies
for 311. 370's type lands in `tcl-dialect` at once, but its
`LexerConfig::default()` rewrites go crate by crate as each is extracted.

Where the designs disagree about order:

- **308 against item 84.** The plan runs 308 after 302 and 303 and
  "alongside item 84", which is stage 11. Split it: 308a (whole-file
  selection from `SignaturePackageRequire`, `Must`/`May` from the structural
  walk) in wave 8; 308b (per position) after 84.
- **391b before the split** would be written into the cyclic `tcl-compiler`
  and moved again. It waits for INT-e.
- **370's** 94 non-test `LexerConfig::default()` rewrites conflict with
  every move if done at once; do them per crate.
- **307's images per crate against 265's test rule:** settled: every image
  is in `tcl-catalogue`, the Tcl core's too (INT-b).

## 8. Contradictions and gaps

| # | Doc A says | Doc B says | Recommendation |
|---|---|---|---|
| C1 | 391: `ProcSummary { params: ParamNeed… }` | 392: `ProcSummary { raises, completion }`; code: `ProcSummary` is purity (`interprocedural.rs:264`) | one `ProcSummary` product with a slot per domain (§ 3); chapter 7's naming table names the record and its owner, `tcl-model` |
| C2 | 308: `LibraryChoice::{Must, May}` | code: `BindingKnowledge::{Must, May}` (`model/binding.rs:211`), `Knowledge::{Proven, BoundedSet}` (`answer.rs:64`); 391, 392: `certainty: Must` | one `tcl_core_types::Certainty`; records keep their own `Absent` and `Unknown` arms |
| C3 | 391c: `TaintColour::CHANNEL` derived from handle types | 393: `CHANNEL` leaves the colours and becomes an origin; code: two `TaintColour`s (`tcl-compiler/src/taint.rs:137`, `tcl-registry/src/taint.rs:50`) | 393's `CoreColour`, `ColourSet` and `TaintTable` in the core; 391c reads origins |
| C4 | 392: `CompletionCodes` in the core | step 5: `CompletionCodeLattice` to `tcl-model`; code: `CompletionCodeDomain` in the core (`completion.rs:189`) | the core's `CodeSet` is the value; `tcl-model`'s lattice is over it |
| C5 | 308: `positioned_context` in `tcl-db` | step 12: the version gate in `tcl-diagnostics`, below `tcl-db`; 305: `check` atoms read `libraries` | computed in `tcl-binding`, memoised in `tcl-db` |
| C6 | 370: `DialectPoint::grammars() -> &'static GrammarProfile` | 370 and spec ch. 4: pack profiles live in the salsa generation; step 14: no `Box::leak` | `GrammarProfileId` plus `Arc<GrammarProfile>` from the generation; compiled-in profiles may stay `&'static` constants |
| C7 | 307, spec ch. 1 and ch. 7 (*Placement*): core images `include_bytes!` in their own crate | 265 rule 1: generic crates dev-depend on `tcl-dialect-tcl` | every fact an image, the Tcl core's included; images in `tcl-catalogue`; surface crates hold hooks only; tests read images at run time through `tcl_registry::testing` (INT-b) |
| C8 | 391: the engine lives in "the analyser" | step 13: `object_types` goes to `tcl-unit`; code: `object_handle_facts` takes a built unit (`object_types.rs:508`) | domain in `tcl-analysis`, solved by `tcl-unit`; records in `tcl-model`; diagnostics in `tcl-diagnostics` |
| C9 | today: taint cascade in `tcl-lsp-db` per procedure (`check_solve.rs:115`) | step 10: taint is a whole-unit pass in `tcl-unit` | one `body_summary` query for every domain |
| C10 | 380: `ebpf_contribution -> Box<dyn AnalyserContribution>` | step 12: `UnitRule`, `SiteRule`, `RuleSet` | `UnitRule`/`SiteRule` |
| C11 | crate-split overview: `bpf-tcl-ir` needs `tcl-analysis` | 380: that edge is unused outside tests | `bpf-tcl-ir` over `tcl-lowering` (INT-k) |
| C12 | 313's hook table: folds in `tcl-registry::value_transfer` | step 10 and 117b: evaluators to `tcl-values` and `tcl-analysis` | the core keeps the declaration; evaluation follows 117b |
| C13 | 313: layout in "the formatting crate" | 289: `tcl-ide-format` | name `tcl-ide-format` |
| C14 | 305's rung table: "4 (dynamic): `tcl-spec-hooks` → `tcl-vm`, linked by the entry point only" and rung 5 as native contributions | spec R2: rung 5 is tabulated answers; contributions are outside the ladder | spec R2; 305's runtime table renumbers |
| C15 | plan: 308 runs after 302 and 303 in the build-time stage | 308: alongside item 84 (stage 11) | split into 308a and 308b (§ 7) |
| C16 | 365: `tcl-sslictcl` builds its model from `tcl-spectcl`'s declaration reader | crate-split overview: `tcl-sslictcl` needs only `tcl-source` | `tcl-sslictcl` depends on `tcl-source` and the reader, not on `tcl-spectcl`'s hook or cache modules; the reader is a feature-free module |
| C17 | 308: "binding coverage: requires in `eval`, `uplevel` or callbacks are invisible" | 390f: callbacks have signatures and are call edges | the callback edges of § 3 reach those bodies; only `eval` of a computed string stays `Unknown` |
| C18 | spec ch. 7: `SpecSource::Image(&'static ImageView)`, `&'static` slices in model types | user packs load at run time; step 14: no `Box::leak` outside `tcl-db` | views only (`SpecRef<'g>`; D2 closed that way, chapter 7); catalogue images are the `'g = 'static` case; a generation owns user-pack bytes in an `Arc`; salsa values hold IDs and copied facts, never a generation |
| C19 | 308: a procedure's entry state is the join of the states at its call sites | this document's first draft: `tcl-binding` computes the context before any call graph exists | entry selections top-down in `tcl-unit` after condensation; before that, end of file with `Must` downgraded to `May` (§ 1) |
| C20 | this document's first draft: "edges only go from unknown to known" | § 3: widening to `unknown_callee` removes proved operands, so edges go both ways | binding frozen inside a component; a counted re-binding budget (§ 3) |
| C21 | this document's first draft: binding reads only `visible_names` | § 2 and § 4: binding reads roles, grammars, transitions, callbacks, embedded ranges and hooks; table rows belong to no command | `binding_fact_slice` and `fact_tables` (§ 5) |
| C22 | this document's first draft (§ 4) and 313: the taint gate is a hook family that keeps its silence | spec chapter 5 and 393: the gate is a `SinkWhen` predicate | one polarity per hook family, declared in chapter 5; no hook family for the gate |
| C23 | spec chapter 3: a full `throws` set is a claim | spec chapter 2: `throws PATTERN+ -when -on` cannot make the claim; real Tcl raises `TCL LIMIT`, `TCL WRITE` and `CHILDSTATUS` beyond any written set | 392's owner: `throws -complete`, and every other set open; § 9's sets are open |
| C24 | § 3: only parameters, returns, callback arguments and instance variables cross bodies | 391:274 and 393:215: handles and taint cross frames through `upvar`, `global`, `uplevel` and renames | 391's owner: summaries record frame effects, and such a call drops every handle in scope to `May` |
| C25 | 392:140: an unknown command raises `TCL LOOKUP COMMAND` and warns | 392:147: an unknown command raises `Any` | 392's owner: `Any`, and no warning |
| C26 | § 3: one walk per body visit | 393:244-245: two seeded runs per parameter, and `param_sinks` has no per-sink transfer | 393's owner: one seeded walk with per-sink keeps and adds |
| C27 | spec chapters 5 and 7: an image naming an ID the binary lacks is stale and recompiled from source | recompiling gives the same dangling ID | that hook is dormant and abstains, reported once; test loaders bind stubs (chapters 5 and 7) |

## 9. Worked example: one procedure through every crate

A procedure that reads a line, opens a log file under `catch`, and writes the
line to it. The line is tainted (`gets` carries `Traits::TAINT_SOURCE`,
`rust/tcl-registry/src/commands/tcl/gets_.rs:83`).

**Source:**

```tcl
proc save {path} {
    set line [gets stdin]
    if {[catch {open $path a} ch opts]} {
        return [lindex [dict get $opts -errorcode] 0]
    }
    puts $ch $line
    close $ch
    return ok
}
puts [save /nonexistent/log.txt]
puts [save out.log]
```

Which crate sees what:

| Crate, pass | Sees |
|---|---|
| `tcl-source` | the body's eleven commands (tclsh's `Cmds 11` agrees), under `tcl9.1`'s profile |
| `tcl-binding` | `gets`, `open`, `catch`, `dict get`, `lindex`, `puts`, `close`, `return` resolved, all `Must`; the `catch` body is a script role; `ch` and `opts` are `VarWrite` roles |
| `tcl-analysis` (SCCP) | `opts` exists only on the error path; the mode word is the literal `a` |
| summary walk | errors: `open` throws `POSIX`, so the catch site's set is `{POSIX}` plus an open remainder (C23); handles: `ch` is a writable `file` on the 0 path only; taint: `line` is tainted and reaches `puts` on a file; return: `POSIX` or `ok` |
| `tcl-diagnostics` | W103; T101 on `puts $ch $line` (393's `Output` sink), which today fires under 8.6 only |
| `tcl-native-plan`, codegen | facts only: `open` and `puts` are effectful, so no folding |

**In memory:**

```rust
// design sketch: ProcSummary and the domain records are proposed here, in 391,
// 392 and 393; the explorer's row and the 8.6 taint facts below are real
ProcSummary {
    body: BodyKey("body:1:0"),
    base: BaseSummary { arity: 1..=1, pure: false, completes: false, .. },
    handles: HandleSummary {
        params: &[ParamNeed { name: "path", accepts: NotAHandle }],
        returns: NotAHandle,
        released: &[],                          // `ch` is opened and closed inside
    },
    errors: ErrorSummary {
        raises: ErrorSet { patterns: &[ErrorPattern(&["POSIX"])],   // from puts and close
                           other: true, certainty: Certainty::May },  // close also raises CHILDSTATUS (C23)
        completion: CodeSet::of(&[0, 1]),
    },
    taint: ProcTaintSummary { returns: TaintTransfer { base: TaintFact::CLEAN, from_params: &[] },
                              param_sinks: &[], validates: &[] },          // 393's gen/kill shape
    returns: Knowledge::BoundedSet { values: vec!["POSIX".into(), "ok".into()], includes_absent: false },
}
CatchFacts { site: Span(36..63),               // tclsh's src range for command 4
             completion: CodeSet::of(&[0, 1]),
             errors: ErrorSet { patterns: &[ErrorPattern(&["POSIX"])], other: true, certainty: Certainty::Must } }
HandleValue { var: (BodyKey("body:1:0"), "ch"), ty: "file", caps: {writable},
              state: Live, certainty: Certainty::Must, on: CatchResult(0) }
```

The `returns` row is the reduced product at work: the errors domain types
`-errorcode`'s first field as `POSIX`, SCCP folds the literal `ok`, and the
caller's `puts` sees a two-value set. The error sets are open (`other:
true`): no written `throws` set is complete (C23), and `close` on a pipeline
raises `CHILDSTATUS`:

```text
$ printf 'a\n' | tclsh9.1 save2.tcl   # save.tcl, then: puts [catch {save {|false}} m o]; puts [dict get $o -errorcode]
1
CHILDSTATUS 31566 1
```

tclsh8.6 prints the same with its own process id. Until 392 gains `throws
-complete`, `returns` is `{POSIX, ok}` only on the paths the open remainder
does not reach.

**Bytecode (tclsh 9.1.0):**

```text
$ printf 'source saveproc.tcl\nputs [::tcl::unsupported::disassemble proc save]\n' | tclsh9.1   # excerpt
      0: level 0, catch, pc 35-54, catch 66
    (10) invokeStk 2
    (15) storeScalar %v1 	# var "line"
    (30) beginCatch 0
    (50) invokeStk 3
    (66) pushReturnCode
    (67) pushResult
    (68) pushReturnOpts
    (69) endCatch
    (70) storeScalar %v3 	# var "opts"
    (76) storeScalar %v2 	# var "ch"
    (83) jumpFalse +40 	# pc 123
    (107) dictGet 1
    (112) listIndexImm 0
    (144) invokeStk 3
```

Instructions 10, 50 and 144 are `gets`, `open` and `puts`. Nothing here
says `ch` is a file or `line` tainted: `storeScalar %v2` stores whatever
`catch` left on both paths. The handle fact needs the catch
result (`jumpFalse` at 83) and the error set together, hence one walk.

**Result:**

```text
$ printf 'a\nb\n' | tclsh9.1 save.tcl; cat out.log     (tclsh8.6 and 9.0 print the same)
POSIX
ok
b
$ printf 'a\nb\n' | tclsh8.4 save.tcl 2>&1 | head -1
wrong # args: should be "catch command ?varName?"
$ printf 'a\nb\n' | ./target/debug/tclvm save.tcl; od -c out.log
NONE
ok
0000000  \n
$ ./target/debug/tcl diag --dialect tcl9.1 save.tcl
save.tcl:3:22: warning W103     open with a dynamic argument (variable or command substitution): if the value starts with "|", it will execute a command pipeline. Validate input or use explicit I/O commands.
diagnostics=1 across 1 input(s)
$ ./target/debug/tcl explore --dialect tcl9.1 --json save.tcl   # interprocedural row, some fields
{"bodyKey": "body:1:0", "name": "::save", "arity": "1..1", "pure": false,
 "returnShape": "unknown", "hasBarrier": true, "hasUnknownCalls": true, "transfer": null}
$ ./target/debug/tcl diag --dialect tcl8.6 save.tcl   # W103 as above, then
save.tcl:6:14: warning T101     Tainted variable $line flows into puts; output may contain injected content
$ ./target/debug/tcl explore --dialect tcl8.6 --show taint --text save.tcl   # taintFacts view
└── function ::save
    ├── line#1: tainted
    ├── line#2: tainted
    └── line#3: tainted
```

- 8.4's `catch` has no options variable: 392's dict shape is per release.
- `tclvm` gives `-errorcode` `NONE` for the failed `open` and reads nothing
  from a piped `stdin`: two VM bugs (§ *Bugs found*).
- Today's row says "a barrier, unknown calls, `returnShape: unknown`"; the
  product says what `save` returns and raises.
- Under 8.6 the tainted line reaches `puts` (T101 on line 6); under 9.1 the
  same file draws no taint at all, 393's bug 1.

## 10. Proposed plan steps

Numbered `INT-a` onwards, to be placed after the plan's renumbering.

- **INT-a. `FactView` and `HookDispatch` in the core** (after 262). Done
  when `tcl-compiler` reads `CommandRegistry` only through `FactView`
  adapters and `install_host`/`install_extension_host` have callers only in
  entry points (`rg`).
- **INT-b. Every fact an image, every image in the catalogue** (with 264 and
  307). Done when item 259's `spec` edit (a comment appended to the Tcl
  core's `.tclspec`) and a release-facts row edit each rebuild nothing under
  `cargo test -p tcl-analysis --lib --no-run`. Under `cargo build --workspace
  --all-targets` they rebuild only `tcl-catalogue`, the binaries, the WASM
  hosts, `xtask` and the `-it` crates that take the catalogue. `rg
  'CommandSpec \{' rust/tcl-dialect-tcl/src` is empty.
- **INT-c. One `Certainty`** (wave 4). Done when `LibraryChoice`,
  `HandleValue` and `ErrorSet` use `tcl_core_types::Certainty`.
- **INT-d. Positioned context in `tcl-binding`** (after 253, 302, 303; 308a
  then 308b). Done when the version gate reads `&ContextIntervals`, `cargo
  tree -p tcl-diagnostics -e normal` lists no `tcl-db`, and 308's rows pass.
  The second half also needs `tcl-unit`'s top-down entry-selection pass:
  308's "inside a procedure" row and a cross-file row pass, and each
  component's re-binding count is recorded.
- **INT-e. The summary framework** (after 258). Port § 3's six engines one
  at a time. Done when the explorer's JSON over the conformance corpus is
  byte-identical across each port, one Tarjan remains, the loops at
  `summary_fixpoints.rs:119,190`, `object_flow.rs:97` and
  `var_escape/interprocedural.rs:273` are gone, and § 3's edit and cold
  budgets hold on release builds (item 106's suite and tcllib 2.0).
- **INT-f. The new domains:** 391b, 392b–c and 470 (393's summaries). Done
  when the examples of 391, 392 and § 9 show their summaries in `tcl
  explore`, and every object-type test passes unchanged.
- **INT-g. Fact slices and pack inputs** (after 286, INT-a). Done when a
  `tcl-db` test edits one `throws` row in a user pack and salsa's event log
  shows `body_summary` re-run only for its callers' bodies, and no binding.
  A second test changes a role (`-role Body`) and sees only the calling
  bodies re-bound; a third adds a `colour` row and sees every summary re-run.
- **INT-h. Grammar profile ids** (with 370, INT-g). Done when every parser
  takes a `GrammarProfile`, pack profiles come from the generation, and the
  memo gate finds no `Box::leak` for one.
- **INT-i. The VM boundary** (with 318). Done when `cargo tree -e normal -i
  tcl-vm` is empty for `tcl-spectcl`, `tcl-explorer`, `tcl-ide-*` and every
  compiler crate, and the server reaches `tcl-vm` only through
  `tcl-spec-hooks`.
- **INT-j. Consumer cones:** § 6's target table as a ledger checked by
  `cargo xtask crate-layer`. Done when the gate passes and item 259 records
  each cone.
- **INT-k. `bpf-tcl-ir` without `tcl-analysis`.** Done when `cargo tree -p
  bpf-tcl-ir -e normal` lists no `tcl-analysis`.
- **INT-l. One taint colour, one completion code value** (with 465). Done when
  `rg 'struct TaintColour|enum CompletionCode(Lattice|Domain)' rust` finds
  one colour and one value type.
- **INT-m. § 9 as a conformance row.** Done when `save`'s summary is checked
  on tclsh 8.6, 9.0 and 9.1.

## Corrections to other designs

- **Crate-split overview:** `bpf-tcl-ir` needs `tcl-lowering`, not
  `tcl-analysis`; drop the edge from the text and the target diagram.
- **Step 10:** SCCP's `ProcedureSummaries` reads the product `ProcSummary`;
  the whole-unit tier's transfers (`taint`, `interprocedural`, `var_escape`,
  `object_types`) come down to `tcl-analysis` as domains as each stops
  reading a built unit, and only the solver stays in `tcl-unit`.
- **Step 13:** `connection_scope` splits between handle lifetimes and
  `tcl-irules`.
- **Step 14:** `body_summary` replaces the taint and optimisation summary
  queries; the inputs gain `CompiledSurfaces` and `UserPack`.
- **265 (registry step 4):** rule 1's macro is
  `tcl_registry::testing::install!`, `tcl-dialect-tcl` holds hooks only, and
  the split crates inherit no known-list row (INT-b).
- **269–272:** `tcl-spectcl`'s `registry_for_dialect_from` takes its
  `SurfaceSet` from the caller and never depends on the catalogue.
- **305:** the runtime-crates table's rung 5 follows spec R2; the loader
  hands image bytes to the core (R27).
- **307 and spec chapter 1:** `tcl-catalogue` embeds every image, and the
  Tcl core's facts are an image (307 decision 1).
- **308:** computed in `tcl-binding`, wrapped by `tcl-db`; split into 308a
  and 308b.
- **313:** folds are evaluated in `tcl-values`/`tcl-analysis` (117b); layout
  is `tcl-ide-format`.
- **370:** pack profiles come from the generation, not `&'static`.
- **380:** `AnalyserContribution` becomes `UnitRule`/`SiteRule`.
- **391, 392:** their `ProcSummary` sketches are slots of one record, with
  `Certainty`; the engines are `tcl-analysis` domains solved in `tcl-unit`.

## Bugs found

- **`tclvm`, `open` failure.** `catch {open /nonexistent/x} m o; puts [dict
  get $o -errorcode]` prints `NONE`; tclsh 9.1 prints `POSIX ENOENT {no such
  file or directory}`.
- **`tclvm`, piped `stdin`.** `printf 'abc\n' | tclvm g2.tcl` with `puts
  [gets stdin line]; puts $line` prints `-1` and an empty line; tclsh 9.1
  prints `3` and `abc`.
- **Taint under 9.x** (confirms 393's bug 1). `proc p {} { set line [gets
  stdin]; eval $line }` draws T100 under `--dialect tcl8.6` and only W101
  under `tcl9.1`; § 9's `save` draws T101 under 8.6 and nothing under 9.1.
- **Interprocedural `hasUnknownCalls`.** `proc f {} { puts stdout x; close
  stdout }` gives `hasUnknownCalls: true` and `hasBarrier: true` in `tcl
  explore --json`, while each statement alone gives `false` for both. So does
  `proc f {} { puts [gets stdin] }`. Every command there is a core command.

## Review fixes (26 November 2026)

- **R1:** decision 2 and § 1's rule table: the Tcl core's facts and release facts are images; `tcl-dialect-tcl` holds hooks only; INT-b measures both edits.
- **R2:** § 1 rows 2 and 4: `tcl_registry::testing::install!`; split crates inherit no known-list row.
- **R3:** § 1 *Views borrow; salsa values copy*; C18; `FactView::command` returns `SpecRef<'g>`.
- **R4:** binding's receipt is `InvocationBinding` (§ 2); the core's view result is `SpecRef<'g>`.
- **R5:** § 1 row 5 and *Corrections*: `registry_for_dialect_from(&SurfaceSet, …)`.
- **R6:** § 1 row 6: an unconditional `tcl_registry::testing` module.
- **R7:** § 1 row 7: the `schema` role.
- **R12:** § 1 *`xtask` is an entry crate*; decision 2, the target graph and INT-b name `xtask`.
- **A1:** § 3: binding frozen inside a component, a counted re-binding budget, and an example; C20.
- **A2 (§ 9 only):** the error sets of § 9 are open, with the `CHILDSTATUS` run; C23. The `throws -complete` fix belongs to 392's owner.
- **A3:** § 5: `binding_fact_slice`, `fact_tables`; INT-g's two extra tests; C21.
- **A4:** § 3 and § 5: summaries keyed per component, `hook_table` and content-hashed profile ids, edit and cold budgets tied to item 106.
- **A5:** § 1 *Entry selections, top-down*, with a termination argument; the pass-order table; INT-d; C19.
- **A8:** § 1, § 4 and the sequence diagram: per-family polarity, and the sink gate is no hook; C22.
- **A9:** § 4: fuel only, and a `values` body runs in step 7's first round only.
- **A10:** § 1: test loaders bind missing native IDs to abstaining stubs; C27.
- **C19–C27:** added from the review's list of missing contradictions.
