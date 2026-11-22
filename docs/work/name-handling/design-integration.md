# Making the designs one design

**Question:** do the design pages on this branch form a cohesive set that shares one design and will work together?

**Method:**
- An independent review read every design page, plan items 1–91 and the [naming API guide](naming-api-guide.md), looking for concepts named or shaped differently, contradictions, missing joins, unvalidated assumptions and phase-order errors.
- The highest-ranked findings were checked against the code.

**Verdict: not yet, and the gaps are specific.** The pages agree on direction: one owner per fact, typed identities, explicit unknowns, binding before consumers, and conformance for every consumer. Ten structural questions were left open or answered differently by different pages, and the phase order in [plan.md](plan.md) scheduled work before the work it depends on. The decisions below close those questions; the pages and the plan are amended to match. Four short validation spikes (plan items 92–95) test the assumptions that have no evidence yet, before the phases that rely on them.

## What the code already settles

- **The per-point name world exists.** `ModuleCommandBindings` (`tcl-compiler/src/command_binding.rs:1449`) already holds, per program point and with `Arc` copy-on-write tables:
  - the command table with may-bindings;
  - namespaces, paths, exports and `namespace unknown` handlers;
  - ensembles and packages;
  - class definitions, object instances and object dispatch;
  - `source_variables: ResolveContext` and the `variable_frame`.

  `same_command_world_tables` already compares two points by allocation. The "`NameWorld`" of the [validation](name-resolution-validation.md) is this structure, tidied: re-keyed by identities, its string sets (`tainted_object_dispatch`, `packages`, `revoked_loaders`) removed, and its contents documented. It is not a new structure.
- **Binding and value analysis already depend on each other.**
  - SCCP and SSA read command binding in 33 files.
  - Command binding uses SSA in 7 files.
  - Binding keeps its own value lattice (`command_binding/cfg_lattice.rs`, `formal_value.rs`, `evaluated_word.rs`).

  So the architecture plan's "binding does not compute values" describes neither today's code nor what binding needs: dynamic heads, `$o m`, `upvar $n` and `uplevel $lvl` need values.

## Decisions

1. **Binding and value analysis are one fixpoint per body unit.**
   - Binding resolves what literal words and its current value facts allow, and leaves an unknown-head marker where they do not.
   - SSA and SCCP run. Sites whose heads, levels or names SCCP has since proved are re-bound, and the loop repeats until nothing changes, with a fixed limit.
   - Both sides use the one abstract-value kernel (item 81). Binding's private lattice (`cfg_lattice`, `formal_value`) becomes a client of that kernel, not a second value analysis.
   - Layers L4 and L5 keep separate owners but are scheduled together per body unit.
2. **Frame and name world.** These replace the different field lists in the command, variable and frame pages.
   - A **`Frame`** is static. It is identified by (body unit, entry context), where the entry context is one of: global, namespace, procedure, method with receiver, lambda, coroutine, level-selected, deferred, or unknown. It carries its namespace identity, its level relation to the caller (same level for `next`), and its receiver.
   - **Analysis is context-insensitive by default:** one frame per body unit and entry context, not one per call. Call-site precision comes only from instantiating contracts (decision 6).
   - Everything that changes inside a body lives in the **name world**: aliases and links (`global`, `variable`, `upvar`, `namespace upvar`, `my variable`, `classvariable`, `link`), traces, path, imports, commands, classes and instances.
3. **Deferred bodies read a deferred point.**
   - Code registered with `after`, traces, `bind` or `interp alias`, or captured with `namespace code`, runs at some later time, not at a program point.
   - Its name world is the join of the worlds at every point reachable after registration, in the global frame. In practice that is the end-of-script world of each file in the workspace assumption, joined with "unknown" for anything an unproven call could change.
   - The potential-execution owner (decision 8) computes these points.
4. **One per-site record: the annotated command.** It replaces the "invocation record", the "forward answer per site" and the `CommandTokens` side record. It is `SourceInvocationBinding` evolved in place, returned by `invocation_at_source(offset)`, so the guide's API stays the entry point ([architecture review](architecture-review.md), resolution 1).
   - Binding produces one for every reached command, however deeply nested.
   - It carries the structured words, the command and variable answers, the selected form, per-word roles, places and value facts, and its result and effects.
   - The reverse index is derived from these records and from nothing else.
5. **Identity types are separate and linked.**
   - **`CommandRef`** is a live command incarnation: allocation site plus incarnation, the existing `CommandIdentity`.
   - A **declaration** (`OriginalDeclarationIdentity`) and a **slot** (namespace plus tail) are separate types, linked from it.
   - Rename edits declarations and the call sites that select them. GVN, purity and folding key on `CommandRef`. Navigation follows `CommandRef` to its declaration.
   - The identity layer (item 77, phase 0) defines all three. Item 35 then moves the analyser and signature-scan models onto them.
6. **Summaries own the inferred facts; the contract is how consumers read them.**
   - The frame-effect summaries (`cfg_builder`) and the interprocedural summaries own inferred effects, purity and constant results.
   - A command contract is assembled from the registry, packs and stubs, and those summaries, in that precedence order.
   - Consumers read only contracts. Summaries are never views of contracts, which removes the circularity in [user and library commands](user-and-library-commands.md).
   - **The class model is built once.** The typed class model of phase 5 is the object-system profile's class fact, populated for the existing families (object systems plan, Order 1–4). Phase 8 only adds new systems and the SpecTcl rewrite.
7. **The answer type keeps the authorities separate.**
   - `Answer<T>` has four states: proven `T`, a bounded set of `T`, absent, or unknown with a reason. Variable answers are `Answer<VariableTarget>`, where `VariableTarget` is `Cell`, `Alias { alias, target }` or `Unset`.
   - An answer carries the **receipts** that support it, each keeping its own bounded authority from one fixed enum (Display, Logical, Native, normal completion, Edit), as the naming guide requires. Display answers may come from written words, for display features only ([architecture review](architecture-review.md), resolution 2). Consumers re-check the receipt they need, and the semantic model never merges receipts into one level.
   - SCCP keeps its own lattice. Its "unknown" (not yet reached) maps to an unreached region, not to `Answer::Unknown`; overdefined maps to unknown.
   - One `AbstractValue` holds exact values, sets, intervals and container shapes, with `ExactValue` as its exact case.
8. **A potential-execution owner.** One owner computes which code is reached, not reached or possibly reached, the deferred points, and which names unknown code may affect. These all depend on it:
   - absence proofs (W123);
   - rename completeness;
   - completeness of the reverse index;
   - whole-program facts (no `tcl::mathfunc` override, no `tcl_precision` write);
   - the deferred points of decision 3.

   It is built with "every body is a unit" (phase 4).
9. **Linearisation recipes are native code, and custom ones are analysis-only.**
   - The shipped recipes (`lookup::c3`, `dfs-keep-last`, `dfs-refuse-repeats`, single inheritance, delegation fallback) are Rust functions in `tcl-syntax` beside `tcloo_linearise`. SpecTcl names a recipe and does not run it.
   - Both engines and analysis call the same functions, so the runtime needs no VM and no crate depends upward.
   - A custom recipe written in Tcl runs only in analysis, in the SpecTcl sandbox. The engines run the user's real Tcl code for a custom object system anyway.
10. **The name world's scope and cost are measured before the design goes further.**
    - Classes and instances stay in the name world, as they are today.
    - Spike 92 measures the memory and time of per-point binding states on the analysis-performance corpus before any persistent-map change.
    - `imbl` replaces whole-table `Arc` clones only if the measurement shows a need.

## Validation spikes

Each is small, measured and reverts if it fails. They run in phase 0, before the phases that depend on them.

- **92. Name-world cost:** memory and time of the per-point binding states on the 687-line file and the corpus. Answers whether per-point state is affordable now that analysis retains about 230 MB.
- **93. Annotated command for nested commands:** produce the record for every reached command in one fixture directory (`oo-dispatch-probe/frames/`) from today's binding and compare it with the existing consumers' answers. Answers whether "computed once" covers nested sites.
- **94. Binding and value fixpoint:** run the loop of decision 1 on the dynamic-head, `upvar $n` and `uplevel $lvl` fixtures in `oo-dispatch-probe/dynamic/`; count the iterations and re-bound sites, and check that fresh and incremental runs give identical, sealed realms.
- **95. Per-body salsa query:** move one consumer (W210 on one body unit) onto a per-body query and measure re-analysis after a one-line edit.

## Corrected order

The dependencies the pages state, applied to the plan:

- **Phase 0, foundations and spikes:** 74–78, 92–95, and the correctness backlog. The backlog is wrong answers only; the writer owner of item 39 moves to phase 2 and only SslicTcl's unbalanced output stays here.
- **Phase 0b:** release facts, registry descriptors and the kernel (79–81). Object-system profile facts for the existing families, with no change in behaviour (86, Order 1–2), move here because frames take their receiver and helpers from profiles.
- **Phase 1:** owners that already exist (22, 31, 43, 60, 61); 91; 13, 14, 32, 33 and 59.
- **Phase 2:** structured words (82); the writer owner and segmentation (39, 41); expression owners (27, 29); index grammar (44).
- **Phase 3:**
  - annotated commands and the binding and value fixpoint (50, decisions 1 and 4, which bring forward the per-site part of 84);
  - frames, namespaces and lookup (19–25, 66, 69–72);
  - the reverse index (67);
  - the workspace index behind the lookup owner's interface as a provider (the first half of 68);
  - typed keys (51–53).
- **Phase 4:** every body is a unit (83, 2); the potential-execution owner (decision 8); 1 (one-member bodies, the same work as 83); 3–5.
- **Phase 5:** one binding walk (the rest of 84); the class fact (86, Order 3–4); rename (35–37); 36's completeness on the potential-execution owner.
- **Phase 6:**
  - the semantic model (85);
  - contracts (54–58);
  - cross-file exports as contracts (the second half of 68);
  - constants (62, 63, 65 after 59);
  - lists and containers (45).
- **Phase 7:** crates and caching (90, 46–49, 73).
- **Phase 8:** new object systems, SpecTcl profiles and value grammars (86, Order 5–7; 87; 6–10; 15–17).
- **Alongside phases 3–5:** engine work (18, 20, 30, 34, 38, 42, 72, 88).

## Remaining wording differences, now fixed

- **Effect descriptors and transitions are one vocabulary.** The registry's existing transitions are extended, and nothing parallel is added.
- **Places:** `PlaceRef` is the interned identity; dense per-unit indices are an implementation detail of a pass.
- **Body units** are issued at lowering by the provenance owner, during the walk that also binds ([structured-procedure plan](structured-procedure-plan.md); [architecture review](architecture-review.md), resolution 3).
- **Counts:** about 42 resolvers is superseded by the inventories' about 60 command and about 70 variable implementations; the frame and namespace choosers are as counted in the [frame page](frame-design.md) and [namespace page](namespace-design.md).

The [architecture review](architecture-review.md) checks these decisions against the branch's earlier ones, resolves six contradictions (decisions 4 and 7 and the body-unit issuer are amended above), and adds plan items 96–99.

Editor actions (rename, references and their MCP and CLI equivalents) add requirements these decisions must meet: binding facts as their own tier per body unit, positioned lookups by index, a persistent workspace tier, `source` as an import edge, scoped completeness and one action engine for every surface ([editor actions](editor-actions-design.md), items 100–106).
