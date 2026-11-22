# Validating the name-resolution design

**Question:** if every plan on this branch is carried out, do command and variable name resolution end up solved one way, with one set of parsers, owners and caches, across the LSP, the compiler and analysers, the VM, the Rust runtime and the tooling?

**Method:** two independent inventories of the code (made without reading these plans) of every production site that handles name text, resolves a command or variable name, chooses a frame or namespace, or caches a resolution result. Each site was then checked against the plans: [command lookup](command-lookup-centralisation.md), [variable lookup](variable-lookup-centralisation.md), [call frames](frame-design.md), [namespaces](namespace-design.md), [rename and alias](rename-alias-design.md), [words and scripts](words-scripts-design.md), [caching](caching-design.md), [analysis data structures](analysis-data-structures.md), [user and library commands](user-and-library-commands.md), [object systems](object-systems-plan.md), the [architecture plan](architecture-tidy-plan.md), the [cross-cutting review](cross-cutting-review.md) and the [naming API guide](naming-api-guide.md). Nine cases where the copies looked most likely to disagree were run under tclsh 8.6.18 and 9.0.4, the analyser, LSP definition and hover, the VM and the runtime ([`name-validation/`](handover/oo-dispatch-probe/name-validation/)).

## Verdict

**Not yet.** The plans give the right owners for the analysis side and leave thirteen gaps. With those gaps open, the plans would still leave a second resolver in the workspace index, three engine resolution loops, a dozen variable-name scanners, and the three LSP matching heuristics behind references, rename and the minifier. Several plans would also build different "frame" values. The amendments below close the gaps; with them, the design is one stack of layers that every component uses.

**Measured:** all nine cases agree with tclsh everywhere they were run:

- a relative `namespace path` entry;
- a namespace procedure that shadows `lindex`;
- an empty ensemble subcommand;
- a scalar named `a(b`;
- `global ::x`;
- `upvar #0 ::g`;
- a non-ASCII byte ending a `$name`;
- TclOO linearisation for an inconsistent order, a deep diamond and an explicit base, in the VM and in the runtime (which uses its own C3 code instead of `tcloo_linearise`).

The copies agree today on these cases, but nothing keeps them in agreement. The faults already measured in the command and variable pages (`next` answered three ways, rename splitting an alias from its target, the W210 family) are where they have drifted.

## What the inventories found

**Command names** (about 60 distinct implementations):

- **Name text is owned.** Only four production sites outside `tcl-syntax` handle `::` by hand, and the `name_delimiters` ledger tracks them.
- **Duplication sits one level up**, in:
  - two candidate encodings in `tcl-syntax` (string `command_resolution_candidates` and byte `NativeNameProjection`) and a third variant, `…_from_namespace_keys`, which treats a relative path entry differently;
  - about 17 resolution sites in the compiler and 11 in `tcl-lsp-core`, several passing no `namespace path` (`alias.rs`, `type_infer.rs`, class resolution, the LSP's class lookup);
  - the workspace index's own link-following resolver (`command_settlement.rs` `resolve_command_target`, `namespace_imports.rs`);
  - three LSP heuristics: definition's `fallback_proc_by_simple_name`, references' `invocation_references_named`, and the minifier's `find_proc_call_sites`;
  - builtins looked up by spelling (`CommandRegistry::get`'s one-level `::` fallback, codegen's `release_admits_arity`, the LSP's builtin gate);
  - two subcommand-prefix rules (`KeywordTable::resolve_bytes`, `tcl_cmd_core::ensemble::resolve_subcommand`);
  - two import-pattern matchers (plain `string_match` against `NativeGlobProtocol`).
- **Engines:**
  - the VM has two lookup loops over the shared projection (`commands/resolution.rs`, `native_name_world.rs`), and the runtime a third (`namespace.rs` `home_of`);
  - the runtime has its own C3 linearisation;
  - the two engines carry separate execution name policies of 970 and 772 lines.
- **Caches:** about 12 string-keyed definition maps in the compiler (analyser types, signature scan, command binding, trusted packages); workspace-index link maps; four salsa queries in `tcl-lsp-db`; a per-URI class analysis in the server; registry lookup tables and context memos; and the engines' epoch- and generation-keyed caches.

**Variable names** (about 70 distinct implementations):

- **Name text is not owned the way command text is:**
  - three name-end and brace scanners with different rules: `ranges.rs` accepts high bytes, `structural_index.rs` always nests, and `rendered_properties.rs` stops at the first `}`;
  - four element splitters (`combined_variable_input`, `split_element_ref_bytes`, BIG-IP's `measured_combined_element`, and the runtime's `compiled_locals` rule);
  - about ten ad hoc `contains('(')` "is an element" tests;
  - the optimiser's text counters;
  - three `static::` rules.
- **Resolution:**
  - three analysis models (the place binder, the analyser's scope tree, the signature-scan symbols);
  - memory SSA's own `MemoryLocation` keys;
  - `var_escape` pinned to the default registry;
  - two instance-variable collectors (the analyser's itcl and snit code against lowering's `class_members`);
  - the LSP choosing a different model per feature (references from `VarDef` with caller-frame data, rename and highlights from variable symbols, dataflow from places);
  - the engines' resolvers, where the runtime's `var_home` has no global fallback but its `vars.rs` does.
- **Frames:** four level grammars (`FrameLevel::parse*` variants, `tcl_cmd_core::frame`, the registry's native-compilation grammar, the runtime's `digits` test).
- **Caches:** `ResolveContext` sets, `VariableCellTable`, SCCP's process-global `frame_clobbers` memo, `connection_scope` label sets, `project_proc_var_index`, `VariableSymbolMap`, and compiled-local tables in four places (codegen, `tcl_bytecode::LocalVars`, the VM, and the runtime's re-derivation).

## Gaps in the plans

1. **No program point in command lookup.** The command plan's owner is `lookup(frame, name, purpose)`; the variable plan's is `variable(frame, name, point)`. Command meaning changes inside a body: `namespace path`, `rename`, `interp alias`, imports, and procedures defined later. The existing `invocation_at_source(head, offset)` is already positioned. **Amend:** both owners take a frame and a program point.
2. **"Frame value" means two things.** The command, variable, frame and namespace plans each list fields for one "frame value". Some of those fields are fixed for an activation: identity, kind, namespace, level relation, receiver. Others are state that binding transitions change at points: declared path, imports, aliases, defined commands, variable links, instance variables, traces. **Amend:** a `Frame` (per activation, interned, with its namespace identity) and a `NameWorld` (the state at a point, produced by the one binding walk from registry transitions and contracts). Lookups read `NameWorld@point` within a `Frame`.
3. **Two answer types and no authority.** The command answer is `Present | MayPresent | Absent | Unknown`; the variable answer is `Cell | Alias | MayBe | Unset | Unknown`. Neither page says how answers relate to the naming guide's bounded authority (Logical advice versus Native execution versus edit permission). **Amend:** one `Answer<T>` (proven, bounded set, absent, unknown with a reason) carrying provenance and authority, specialised to `CommandTarget` and `VariableTarget`. Command answers extend `SourceInvocationBinding`; variable answers extend `SsaSourceView::read_reference`, so no parallel receipt layer appears.
4. **No reverse index.** References, rename, call hierarchy, code lens, highlights and the minifier need "every site that names X". The plans list them as consumers of a forward lookup, but today each of them matches names itself. **Amend:** a reverse index per identity, derived only from the forward answers at every site (proven and possible kept apart). The three LSP heuristics and the per-feature variable models are deleted, not ported.
5. **No design for the cross-file name world.** Nothing says how procedures, namespaces and variables from other files, `source`d files and packages enter `NameWorld`, and the workspace index resolves links itself. **Amend:** each file and package exports its effects as contracts and transitions ([user and library commands](user-and-library-commands.md)). The binding walk applies them where a `source`, `package require` or workspace assumption brings them in. The workspace index stores exports and the reverse index, and never resolves.
6. **Two candidate encodings with no chosen survivor.** The command plan forbids calling the candidate primitives outside the owner, but does not choose between the string family and the byte projection, which differ on relative path entries. **Amend:**
   - the byte `NativeNameProjection` with its per-interpreter recipes (C Tcl releases, Jim) is the only candidate generator;
   - relative `namespace path` entries are made absolute once, when the path is bound, as Tcl does;
   - the string family becomes a display adapter and is then deleted.
7. **Variable name text is not in any plan.** The command side already has owners and a gate; the variable side does not. **Amend:**
   - one variable-reference grammar per release and dialect in `tcl-lexer` (`scan_var_ref`: name end, braces, index), used by the structured words;
   - one element splitter taking the selected protocol (NUL handling, BIG-IP's rules);
   - one `static::` rule from the iRules profile;
   - a drift gate on `contains('(')`, `find('$')` and hand-written brace scans of variable text.
8. **Builtins by spelling.** Registry semantics are looked up by the written word in several places, ignoring a procedure that shadows the builtin. **Amend:**
   - registry facts are reached only through a resolved `CommandRef` that names a builtin (annotated commands, plan item 50);
   - `CommandRegistry::get` loses its `::` fallback for analysis callers;
   - one subcommand-prefix rule in the registry, used by `tcl-cmd-core`.
9. **Engine resolution is only half planned.** The frame and namespace plans cover the engine traits and alias registration, but not:
   - the three lookup loops;
   - the two execution name policies;
   - the runtime's own C3;
   - the runtime's level test;
   - the compiled-locals rule held in four places.

   **Amend:**
   - one lookup loop per engine, written once over a `tcl-runtime-api` namespace-walk trait;
   - one execution name policy in `tcl-syntax`, with per-engine adapters for live objects only, as the naming guide's Jim example does;
   - `tcloo_linearise` (later the object-system profile's linearisation) in both engines;
   - one level grammar;
   - one compiled-locals decision in the registry, read by codegen, the bytecode, the VM and the runtime.
10. **The engines' caches are not placed in the cache design.** Salsa is the analysis cache. The engines' epoch and generation caches are a legitimate separate tier, but they should be invalidated by the same identity tokens (incarnation and generation) that `tcl-runtime-api` exposes to analysis, not by private counters. **Amend:** say so in the caching plan, and require both engines to invalidate on the same events (`rename`, `namespace delete`, `namespace path`, import, `interp alias`, TclOO redefinition).
11. **The name caches have no target layout.** **Amend:** exactly three analysis tiers, all salsa queries:
    - per-body binding facts (`Frame`s, `NameWorld` transitions, forward answers per site);
    - per-file and per-package exports (contracts, cached on disk for installed packages);
    - the reverse index per identity.

    Delete everything else listed above: the analyser and signature-scan string maps, workspace link maps, `VariableSymbolMap`, the per-URI class analysis and the `frame_clobbers` memo.
12. **The phase order leaves two owners live.** The architecture plan builds the lookup owners in phase 3 but folds the analyser walk and the signature scan into binding only in phase 5. Between them, both would still resolve names. **Amend:** in phase 3 the analyser walk and the signature scan stop resolving and become producers of declarations for binding; phase 5 removes them.
13. **Tools are missing from the consumer lists.** **Amend:** add to the command and variable consumer lists, and to consumer conformance:
    - the minifier (`name_compaction`, `aliasing`);
    - codegen (`defined_in == "::"`, inlining frames, `release_admits_arity`);
    - MCP (`def_use_chains`, `irule_gen`'s `static::` rule);
    - graphs, the explorer, the diagram, SpecTcl's static drive and the iRules test generator.

Also folded into existing items: `var_escape`'s default registry, memory SSA keys from places, one instance-variable collector from the object-system profile, and the namespace plan's glob item extended to the import-pattern matchers.

## The resulting design

| Layer | Owner | Used by |
| --- | --- | --- |
| Name text | `tcl-syntax` naming (qualified names), `tcl-lexer` variable-reference grammar and element splitter, per release and dialect | everything, including both engines |
| Rules | `NativeNameProtocol` per interpreter: candidate order, variable fallback, level grammar, prefix rule, glob, linearisation (object-system profiles), compiled-locals rule | binding, both engines, codegen |
| Identity | `CommandRef`, `NamespaceRef`, `PlaceRef`/cell, `FrameRef`, `InstanceRef`, `BodyUnitId`, each with an incarnation; engine tokens through `tcl-runtime-api` | every layer below the consumers |
| State | `Frame` per activation; `NameWorld` at each point, from registry transitions, effect descriptors, contracts and other files' exports | the two lookup owners |
| Owners | `command(frame, point, name, purpose)` and `variable(frame, point, name, purpose)` returning `Answer<T>` with provenance and authority | the semantic model |
| Indices | forward answers per site; reverse index per identity; per-file and per-package exports (salsa; packages on disk) | references, rename, hierarchy, lens, highlights, workspace diagnostics, minifier |
| Consumers | diagnostics, LSP features, optimiser, codegen, MCP, CLI, explorer, diagram, test generators | — |
| Engines | the same text, rules and identity vocabulary; their own live world and caches invalidated by shared tokens | execution |
| Conformance | one vector table (frame and body kind, mutation, release) run through both engines, binding, every analysis and LSP consumer and the tools, against tclsh and Jim | gates |

Nothing above the rules layer reads names as text, and nothing below the consumers caches a resolution outside salsa or an engine's token-invalidated tier.

## Plan

Items 66–73 in [plan.md](plan.md); the command and variable pages carry the corrected owner signature and point here.
