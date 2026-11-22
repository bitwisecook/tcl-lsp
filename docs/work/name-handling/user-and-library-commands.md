# User procedures, objects and libraries in the typed IR

[Analysis data structures](analysis-data-structures.md) proposes annotated commands: each command node carries its resolved identity, form, per-word roles, places, types and body units, and its result and effects, computed once. For built-in commands those facts come from the registry. This page covers the commands the registry does not ship: procedures and classes in the user's own program, libraries in the workspace, installed source packages, and binary or generated extensions.

**Principle:** every command gets the same facts — a **command contract** — whatever its origin, so no consumer special-cases user code. Contracts come from a fixed set of sources, merged with explicit precedence and provenance, keyed by command identity rather than name, and recomputed incrementally when their source changes. Where no source can answer, the contract says unknown and consumers abstain.

## The command contract

One shape for every command, the same shape the registry already describes for built-ins:

- **identity:** a `CommandRef` from the identity layer — for a procedure, its allocation site and incarnation; for a library command, the package identity and version, the qualified name and the declaration site;
- **forms:** arity, formals with defaults and `args`, subcommands or methods, options;
- **per-parameter roles:** value (with type or domain), variable name (and which frame it names: caller at level *n*, global, namespace), script body (and its evaluation context), expression, command prefix or name, index, option;
- **result:** type, shape and domain;
- **effects:** variables read and written in which frames, commands defined, renamed or deleted, instances created or destroyed, namespaces changed, I/O, errors raised, purity;
- **provenance and confidence:** where each fact came from (registry, pack, stub, inference) and whether it is proven, declared or unknown.

## Sources, in precedence order

1. **The registry** for built-ins, per release and dialect.
2. **Shipped packs** (SpecTcl) for known libraries — tcllib, Tk, itcl, snit, F5 — and object-system profiles ([object systems](object-systems-plan.md)).
3. **Declared contracts in the project:** a workspace SpecTcl pack, sidecar `.tcl.stubs` files and inline `# tcl-lsp: stub` blocks. These are authoritative for what inference cannot see: binary and critcl extensions, commands created dynamically, deliberate API contracts.
4. **Inferred contracts from source**, through the same pipeline as everything else:
   - user procedures and lambdas in the document;
   - procedures in other workspace files;
   - **installed source packages** found by package discovery ([packages](package-design.md): `pkgIndex.tcl`, `tclIndex`, Tcl modules), analysed once per package version.

   Inference uses what the pipeline already computes: formals for arity; parameter traits (`upvar 1 $p` → variable name in the caller's frame; `uplevel 1 $p` or `eval [list $p]` → script body in the caller's frame, which is the [overrides package](overrides-package-review.md) case; `{*}$p` or `after 0 $p` → command prefix; `expr $p` → expression); value domains and types of parameters from their uses; result types from returns; frame effects from the `cfg_builder` summaries; purity and taint from the interprocedural summaries.
5. **Objects:** a class defined in user or library code is turned into a class fact by its object-system profile, and each method, constructor, forward or delegated method gets a command contract of the same shape, with the receiver as an implicit parameter. Instances are values in the handle lattice, so `$obj method` resolves through the class fact to a method contract.
6. **Unknown:** anything else — a command name built at run time, an `unknown` handler, a package with no source and no declaration — gets an explicit unknown contract.

When a declaration (3) and an inferred contract (4) both exist, the declaration wins for the contract consumers see, and the inferred one checks it: a diagnostic reports a declared arity, role or effect that the implementation contradicts.

## Identity, context and change

- **Identity, not names.** Contracts attach to command identities, so a procedure redefined, renamed, imported, aliased or shadowed in a namespace gets the right contract at each call ([rename and alias](rename-alias-design.md), [command lookup](command-lookup-centralisation.md)); an unproven redefinition makes the contract unknown from that point, not wrong.
- **Context-relative effects.** A library procedure's effects are expressed relative to its caller (writes to the caller's frame at level 1, to the global namespace, to the caller's namespace), and instantiated at each call site with the caller's frame value ([call frames](frame-design.md)).
- **Recursion and cycles.** Contracts for mutually recursive procedures are computed per strongly connected component of the call graph to a fixpoint, widening to unknown when they do not settle.
- **Incremental.** A contract is a salsa query keyed by the body unit (or the package source and version), so editing one procedure re-runs its contract and only the callers whose annotations depend on what changed ([caching](caching-design.md)). Installed-package contracts are cached on disk by package source hash and dialect, so a library is analysed once across sessions and projects.
- **Dialect.** Contracts are computed for the selected dialect and release; a library whose behaviour differs by release gets per-release contracts.

## One format for declared and inferred

The `spec-author` skill and Spec Studio's `import_package` already turn a library's source into SpecTcl from the compiler's inference. Make that the same data the analyser uses: an inferred contract can be **exported as a SpecTcl pack**, reviewed, edited and committed, at which point it becomes a declared contract (source 3) and inference keeps checking it. Users who want a stable, documented API for their library get one without writing Rust; users who do nothing still get inferred contracts.

## What works today and what does not

Measured in the other studies:

- **Works:** arity checks for user procedures, including through aliases and renames (E002); navigation to user procedures through `namespace path`, imports and renames; inferred parameter traits exist and drive the LSP's caller-frame navigation.
- **Does not:** W210 ignores a callee's `upvar` writes (false positive); parameters passed to `uplevel 1`/`eval` are not recognised as caller-frame scripts, so their contents are unchecked ([overrides](overrides-package-review.md)); method bodies get no dataflow; objects named or returned in other ways lose method checks in nested calls; contracts of installed source packages are not computed (the package resolver scans indices for navigation, not analysis; to be measured); commands from binary extensions are unknown unless stubbed; there is no check that a declared stub matches the implementation.

## Integration with the naming architecture

The [naming API guide](naming-api-guide.md) and the [consumer matrix](../../design/contracts/naming-consumer-matrix.md) define how names are handled on this branch: shared owners per naming purpose, typed answers that keep their original source and context, and **receipts** whose authority is explicitly bounded (Logical source advice, Native execution, Normal completion and edit permission are separate and never implied by one another). Command contracts must be one more consumer and producer inside that model, and should consolidate receipts that already exist rather than add a parallel layer.

**Where a contract enters the naming flow:**

1. **Input:** the contract query runs under the actual `ResolvedAnalysisInput` — its `ContextRegistry` (registry, packs, environment, availability), full `LexerConfig` and unit profile. Packs and declared stubs reach it only through `ContextRegistry` (`Provenance::Document` for inline and sidecar stubs, pack provenance for packs); nothing rebuilds a registry from a dialect name.
2. **Selection:** a call site's contract is found by the command-binding owner, never by name: `invocation_at_source(head, offset)` → `proved_target()` (or the possible targets) → the command identity → the contract for that identity. An unproven or possibly absent target yields an unknown contract, exactly as the binding answer does.
3. **Declaration:** a procedure's inferred contract hangs off its publication receipt (`OriginalSourceProcedurePublications`, conditional and moved slots included) and its declaration identity (`OriginalDeclarationIdentity`), so known delete, replacement and rename withdraw or move it with the declaration.
4. **Parameters are naming purposes:** each role in a contract names the owner that interprets the argument at the call site —
   - a variable-name parameter carries a frame relation (`FrameLevel`: caller at level *n*, global, namespace) and is resolved by the variable binding and place owners in the caller's frame value;
   - a script-body parameter carries an evaluation context (caller frame, named namespace, global at call time, level-selected, object frame) and becomes a body unit under that frame;
   - a command-prefix or command-name parameter carries a lookup purpose (`NativeNameProtocol` lookup, with the target lookup rule `AliasTargetLookup::Global` or `CallerNamespace`) and is resolved by the command lookup owner when, and in the frame where, it is evaluated;
   - an index, option or typed value parameter is read by the selected grammar owner (index, option table, value grammar) for the selected dialect.
5. **Effects are naming transitions:** a contract's effects are expressed as the registry's existing transition vocabulary — command publication, rename and deletion through the command-binding transitions and `NativeNameProtocol` publication and rename purposes; namespace creation, import and path changes through `NamespaceTransition`; variable creation and links through the variable-effect descriptors; instance creation through object-system profiles. Command binding applies them at the call site as it already applies a built-in's transitions, so a user procedure that defines, renames or aliases commands updates the command world the same way `proc` and `rename` do.
6. **Authority:** an inferred contract is Logical source advice. It can drive diagnostics, navigation, completion and hover; it grants no Native execution, Normal completion, physical cell or edit permission. Optimisations and edits that rely on it still need their own admission receipts (for example, the rename owner's coverage checks and the optimiser's equivalence checks). A declared contract carries the same authority as the pack or stub that declares it.

**What it consolidates** (the summaries stay the owners of inferred facts and the contract is assembled from them, never the reverse; see [design integration](design-integration.md)): several existing receipts are partial contracts and should become views of the one contract, each keeping its bounded authority — `SourceDeclarationSignature`, `ItemSig` and `FormalArgumentCountShape` (forms and counts), the parameter traits and `ProcDef` caller-frame parameters (roles), the LSP's caller-frame bindings (variable-name roles in use), `OriginalSourceCallbackProcedureLookup` and authored command-prefix data (command-prefix roles), the interprocedural summaries (effects and purity), and the declared semantics of packs (`DeclaredSemantics`).

**What it must not do:** turn a display name back into identity; let an inferred contract stand in for an execution or edit receipt; or let a whole-file or workspace declaration list stand in for a positioned binding answer (the guide's rule that an unresolved-name list cannot prove a project declaration occupies the call's slot).

**Gates and registration:** each contract interface gets an owner row in the [shared utility contracts](../../design/contracts/shared-utility-contracts-rust.md) manifest (checked by `owner-resolution`) and a row in the consumer matrix stating its authority; the conformance matrix runs the contract's facts against tclsh for the fixtures in [`oo-dispatch-probe/`](handover/oo-dispatch-probe/README.md).

## Plan

1. **One command-contract type** shared by the registry, packs, stubs and inference, carried on annotated commands (plan item 50) and keyed by command identity.
2. **Inferred contracts as queries:** roles from parameter traits (including caller-frame scripts and command prefixes), types and domains from uses, results from returns, effects from the frame-effect and interprocedural summaries, per call-graph component; consumed by W210, W123, E002, navigation and the optimiser.
3. **Library contracts:** analyse installed source packages found by package discovery, per version and dialect, cached on disk by source hash; declared contracts for binary and critcl extensions through packs and stubs.
4. **Objects:** class facts from object-system profiles produce method contracts of the same shape, for user and library classes alike.
5. **Naming integration:** select contracts through the command-binding owner's positioned answers; hang inferred contracts off publication and declaration receipts; express parameter roles as naming purposes (frame relations, evaluation contexts, lookup purposes, grammar owners) and effects as the existing transition vocabulary; keep inferred contracts as Logical advice; fold the existing partial receipts (declaration signatures, parameter traits, caller-frame bindings, callback targets, interprocedural summaries, declared semantics) into views of the contract; register owner and consumer-matrix rows.
6. **Declared versus inferred:** export inferred contracts as SpecTcl through `spec-author` and Spec Studio; report contradictions between a declaration and its implementation.
