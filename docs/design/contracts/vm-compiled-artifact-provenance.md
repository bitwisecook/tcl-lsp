# TclVM compiled-artifact provenance and invalidation

This contract covers runtime compilation in `tcl-vm` and every reusable or
deferred bytecode activation it creates.

## Owner

`rust/tcl-vm/src/compiled.rs::CompiledUnit` is the single owner of a compiled
function and the VM-local facts that authorised it:

| Field | Meaning |
|---|---|
| `asm` | The `FunctionAsm` to execute. |
| `source_namespace` | The namespace the unit was compiled in.  A frame whose current namespace differs is stale (`cannot continue bytecode after namespace changed`), except a scanner-only `foreach`/`lmap` driver. |
| `profile_generation` | The selected dialect grammar and command surface. |
| `command_epoch` | The command and inlined-procedure source bindings, selected targets, and trace mode last validated for the unit. |
| `compiler` | Either the `CompileService` generation that produced the unit or the generation at which an embedder-owned artifact was explicitly admitted as foreign. |
| `manifest` | The `ArtefactIdentityManifest` of the module the unit came from, so the rungs a disagreeing field rests on are refused again whenever the unit is re-checked. `None` for a plain-dispatch child and for a scanner placeholder, which rest on nothing the manifest covers. |

`Vm::compiled_unit` is the production path for VM-compiled assembly;
`Vm::admitted_foreign_unit` is the explicit public-artifact admission path. A
consumer must carry the complete unit into `Frame`; it must not combine old
assembly with the VM's current generations when an activation is pushed.

Procedure bodies, TclOO methods, `FunctionHandle`, coroutine activations,
cached/deferred eval, catch, try, substitution brackets, and runtime
`foreach`/`lmap` bodies all use this owner. Module caches may retain a
`ModuleAsm`, but consumption turns its validated top-level function into a
unit before execution.

`ModuleAsm::procedure_provenance` identifies a reusable procedure by its exact
canonical rooted constructed key, raw formal-parameter value, and raw body
value. Compiler keys are already constructed identities, not written Tcl
names: the VM removes exactly the leading `::` root marker and never sends the
remainder through written-name canonicalisation. This distinction preserves a
literal `:` namespace (`:::::p` rooted, `:::p` in the VM) separately from the
global `::p`. Multiple exact definitions of one procedure name may coexist in
the admission cache, and a provenance name that differs from its module map key
is rejected.

Public `run_module` preserves the supplied module's top-level bytecode
semantics. Because the VM cannot prove which compile service produced an
embedder-owned `ModuleAsm`, it admits the top level and procedures as foreign
rather than stamping them as current. A source-bearing foreign procedure is
recompiled through the current service on first entry. When no service is
installed, a procedure admitted at the current generation may execute as
supplied if its profile and command bindings still match; it stays foreign so
a later `set_compiler` forces recompilation. Modules returned by the VM's own
compile cache use the current-service production path directly.

## Compile target

A runtime compile target has three independent identities:

1. the dialect profile supplies lexer, expression, release, and availability
   semantics; and
2. a procedure target supplies its exact formal local-variable table and
   canonical runtime namespace; and
3. the compile service supplies the registry and compiler implementation.

`tcl-runtime-api::ProcedureCompileTarget` is the typed procedure-only entry.
It must not fall back to script compilation: that would lose local parameter
slots, procedure `return` semantics, namespace resolution, and nested-procedure
provenance. `ProcedureDispatch::Plain` is a fail-closed capability request; the
VM verifies the returned module is actually plain even when the request was a
retry after live command-binding validation rejected an optimised candidate.
When the compiler inlines a user procedure, `FunctionAsm::procedure_bindings`
carries the source invocation's unrooted constructed resolution namespace and
command spelling, the selected procedure's exact rooted constructed key, and
its raw parameters and body into the reusable artifact. VM admission first
resolves that source invocation again and requires the same selected key, then
checks the exact definition. This rejects both a changed definition and a new
namespace-local or namespace-path command shadowing an otherwise unchanged
target. Constructed namespaces lose exactly one leading `::` root marker and
are never passed through written Tcl name canonicalisation.

A specialised command site records `FunctionAsm::command_bindings`: the
source spelling, its resolution namespace, and the registry identity the
site was compiled for. For a spec-pack command that declares `alias_of` and
carries its target's own codegen stamp, that identity is the target's
(`ResolvedCall::stamp_identity`), so admission follows one prefix-free
`interp alias` hop from the pack spelling to the builtin; a proc, a native
command, or anything else at the pack spelling refuses the site.

A site whose emitted code rests on a spec pack's facts also records a
`tcl_runtime_api::SiteClaim` in `FunctionAsm::site_claims`: `PackFacts` for a
constant a pack-supplied spec's `const_fold` computed at compile time, and
`BuiltinAlias` beside a binding whose identity came through `alias_of`. Each
carries the pack's `PackFactStamp` — its name, the content hash its snapshot
key interns, the loader's vocabulary version, the registry overlay
generation, and the evaluator revision the site compiled under. A pack may
claim; only the VM attests: admission (`function_command_bindings_match`)
requires every claim's stamp to be one the VM holds, compared whole.
`Vm::set_pack_facts` sets them; an embedder that compiles against a pack set
hands the VM `tcl_spectcl::PackSet::fact_stamps` for that set. A VM holding
no facts, the default, admits exactly the units that claim nothing — every
unit compiled without a pack — and refuses the rest like any other failed
binding: plain dispatch when the unit carries source and a compile service
is installed, an admission error otherwise.

`ReferenceBody` is the claim of a procedure binding the compiler took from a
pack: a command a pack declares `TclBody`-backed has its definition inlined into
the procedures that call it (`tcl_compiler::inlining::inline_reference_bodies`),
which records the binding as it does for a procedure the module defines and
codegen records the claim beside it, naming the binding, the kind of backing
(`tcl_runtime_api::BackingKind`) and the pack's `PackFactStamp`. Admission holds
the live command to the binding as it does any other, and beside the held stamp
requires the claim's backing to be `TclBody` and its binding to be one the
function carries (`site_claims_hold`): an exact match of a procedure's text says
nothing about whether the command *is* that procedure, and the VM defines none
from a claim. A definition's text is the spec's own for `PackText` and the file
the loader read at load for `PackageSource`; the compiler reads no file, and the
text it copies is appended to the compile's source and is no part of the
artefact's (`ModuleAsm::source` is the module's own).

Every module a compiler emits states an `ArtefactIdentityManifest`
(`ModuleAsm::manifest`, `tcl_runtime_api::manifest`): the ABI version
(`CODEGEN_ABI_VERSION`, a fingerprint of `CodegenAbiImportId`'s table), the
environment, release and build of the profile the module carries, the package
floors in force, the pack facts any function's sites claim (the claims'
stamps, sorted and without repeats), the hash of the intrinsic table the
emitter keyed against (`tcl_registry::intrinsic_table_hash`), and the revision
of the Tcl library both runtimes embed. The VM states the same fields of
itself (`Vm::held_identity`): the `RuntimeContext` it is pinned to, the pack
facts it holds, and this build's tables. `Vm::pin_context` pins a context
resolved through `tcl_registry::model::ingress`, the ingress the compiler
uses, where an overlay nothing has installed is a `PinError` and never the
un-overlaid generation under another name; the generation at the context's
overlay is held for as long as the pin stands. `Vm::set_dialect_profile` is
the profile form of the same pin.

The manifest is checked as a whole, and a field that disagrees refuses the
rungs that rest on it and no others:

| Field | Refuses |
|---|---|
| `abi_version`, `environment`, `release`, `build` | every rung, since they decide what every word of the unit decoded to: the module is not run, and the error names the field and both values (`validate_module_profile`) |
| `packages`, `packs` | rungs 1 and 2: a function with a pack-fact or builtin-alias claim |
| `intrinsic_table_hash` | rung 4: a function with command bindings, whose specialisations rest on a shipped implementation's identity |
| `embedded_stdlib_revision` | rungs 3 and 4: a function with procedure bindings, whose body may have been resolved from the library, and one with command bindings |

Every pack the artefact states must be one the VM holds, and a VM may hold
more; every other field must be equal. A function's rungs are
read off what it records (`FunctionAsm::rungs`), and a refused function is
recompiled plain or, without source or a compile service, an admission error,
as for any other failed binding; a function with only generic-dispatch sites
is admitted under a changed pack set. Assembly no compiler emitted, which
carries no manifest, is admitted by its profile, bindings and claims alone.

`BytecodeCompileService::for_profile` follows the profile's shared registry.
`BytecodeCompileService::new(custom_registry)` owns the embedder registry and
keeps it when `compile_for_profile` selects the profile grammar. Profile
selection must not silently replace that custom registry.

The VM cannot compare arbitrary trait objects for semantic equality. Every
call to `Vm::set_compiler`, including default-to-custom, custom-to-default, or
two services for the same profile, therefore advances `compiler_generation`.

## Invalidation

| Mutation | Cached modules | Source-bearing reusable unit | Live/suspended frame |
|---|---|---|---|
| Dialect/profile | Clear | Recompile lazily | Fail closed |
| Compile service | Clear | Recompile lazily | Fail closed |
| Command/trace epoch | Revalidate, or compile plain dispatch | Recompile or revalidate lazily | Redispatch at a source-command boundary |
| Pack facts (`set_pack_facts`) | Revalidate, or compile plain dispatch | Recompile or revalidate lazily | Redispatch at a source-command boundary |
| Runtime context (`pin_context` to another context under the same profile) | Revalidate, or compile plain dispatch | Recompile or revalidate lazily | Redispatch at a source-command boundary |

`set_compiler` clears both eval caches and `module_procs`. Procedures,
methods, and function handles retain source and recompile on their next entry.
A suspended coroutine has a program counter and operand stack and cannot be
reconstructed from source without changing continuation semantics, so it
fails with `cannot continue bytecode after compile service changed`.

Deferred bytecode already owns its original unit. If a mutation occurs before
that unit is activated, frame admission rejects it rather than stamping it as
current. Scanner-only `foreach`/`lmap` drivers may advance because their list
grouping and variable writes are invariant; the stored body keeps its own
unit and is checked when pushed.

## Tick boundary

The trampoline checks profile and compiler generations both before and
immediately after every `tick`. The second check is required because a native
command can replace the profile or compile service and return from the final
instruction of a frame. Settlement, including inline and runtime-dispatched
catch/try, trace callbacks, tail calls, and coroutine suspension, must not make
that old frame appear to have completed safely. Before a coroutine resumes or
accepts `Suspend`, it validates the whole frozen stack so a newly compiled
handler or `finally` activation cannot hide a stale ancestor.

`Vm::install_tick` is the exhaustive owner for turning a dispatch result into
an activation, a completion, a tail call, or a suspension. The ordinary drive
loop and a nested `tailcall` target both use that owner; neither may recognise
individual deferred commands or install `Tick::Call`, `PushScript`,
`PushCatch`, `PushSubst`, `PushEachLoop`, or `PushTry` independently. Every
pushed frame takes the issuing command's pending execution-leave context at
that boundary. A tailcalling procedure settles its own leave context before
the replacement command enters, while transparent `eval`, `uplevel`, and
`apply` wrappers settle with `RETURN` and retain their normal namespace and
temporary-command cleanup.

Scanner activations that need an otherwise-empty `CompiledUnit` (`subst` and
runtime-dispatched `foreach`/`lmap`) stamp that placeholder when the command
produces its tick, not later when the frame is installed. The placeholder
therefore carries the exact source namespace, profile generation, command
epoch, and compile-service generation that authorised the deferred work.

Stale failure travels through the ordinary completion-settlement path so
inline exception machinery sees a command-like error. Unwind validates its
current activation before the first pop and revalidates every parent crossing
before accepting even an OK or `return` completion, so neither a synchronous
tail-call target nor a fresh handler can pop a stale activation. A rejected
coroutine installs its frozen flow and unwinds normally before teardown,
firing applicable unset and execution-leave traces exactly once and retiring
the coroutine and temporary-lambda commands through the command-lifecycle
owner so their delete traces also fire exactly once. Queued injections do not
run.

## Compatibility gate

Changes to `tcl-vm`, `tcl-compiler`, the registry, or SpecTcl execution are in
the path set for `make test-spectcl-compat`. That gate must continue to run the
frozen SpecTcl 1.x loader fixtures and the 2.0 executable-pack suite. Compiler
provenance changes must not bypass the central host bootstrap introduced by
#1766 or narrow either compatibility line.

## Focused witnesses

`rust/tcl-vm/tests/command_mutation_deopt_e2e.rs` covers:

- Tcl 9.0.4 procedure-call resolution after a namespace-local command shadows
  the unchanged global procedure whose body had been inlined;
- default/custom registry swaps for cached eval, `FunctionHandle`, procedures,
  TclOO methods, embedder-owned module procedures in both operation orders, and
  a self-contained source-less AOT module;
- fail-closed suspended coroutines before injection, exactly-once stale
  cleanup traces, stack-wide handler/`finally` suspension or return, and
  non-OK tail-call settlement after a compiler swap;
- terminal profile changes from inline and computed-head catch/try plus
  variable-trace paths; and
- a module that claims no pack facts admitted under a VM holding facts for
  a changed pack set (`a_rung_zero_module_is_admitted_under_a_changed_pack_set`);
- the manifest's per-rung check: a manifest listing a pack the VM does not hold
  refuses the unit with a pack-fact site and not the unit with only
  generic-dispatch sites that carries it
  (`a_manifest_disagreeing_on_packs_refuses_only_rung_one_sites`), a unit that
  rests on no pack is admitted under every pack set
  (`a_rung_zero_unit_is_admitted_under_a_changed_pack_set`), another ABI,
  environment, release or build refuses the whole module
  (`a_manifest_for_another_world_refuses_the_whole_module`), and another
  intrinsic table refuses the unit whose specialisations rest on a shipped
  implementation and no other
  (`a_manifest_for_another_intrinsic_table_refuses_only_shipped_backing_sites`),
  a pin to other package floors refuses the unit that rests on a pack
  (`a_pin_to_other_package_floors_refuses_the_units_that_rest_on_packs`), and
  a pin made part-way through a running function is seen at its next command
  (`a_running_function_is_checked_against_its_manifest_when_the_pin_changes`).

`rust/tcl-vm/tests/cross_version_command_surface_e2e.rs` covers the pin: the
profile form is the context the profile names, the VM's held identity is the
identity a module compiled for its pin states, a context the ingress refuses
leaves the pin unchanged, and the `trace` option table is the pinned
profile's and not the plain release its runtime version names.

`rust/tcl-spectcl/tests/codegen_stamps.rs` covers a bundled spec pack's
`alias_of lassign` command: its specialised site records `lassign`'s
identity and the pack's facts, the VM holding those facts admits it through
the alias hop with no plain recompile, and a proc at the pack spelling, or
facts for a changed pack, is refused and recompiled plain. A bundled
pack's `llength` override folding a constant through its `const_fold` is
admitted only while the VM holds the pack's facts.
