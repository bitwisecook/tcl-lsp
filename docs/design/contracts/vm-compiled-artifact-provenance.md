# TclVM compiled-artifact provenance and invalidation

## Status

Implemented. This contract applies to runtime compilation in `tcl-vm` and to
every reusable or deferred bytecode activation it creates.

## Owner

`rust/tcl-vm/src/compiled.rs::CompiledUnit` is the single owner of a compiled
function and the VM-local facts that authorised it:

| Field | Meaning |
|---|---|
| `asm` | The `FunctionAsm` to execute. |
| `profile_generation` | The selected dialect grammar and command surface. |
| `command_epoch` | The command and inlined-procedure source bindings, selected targets, and trace mode last validated for the unit. |
| `compiler` | Either the `CompileService` generation that produced the unit or the generation at which an embedder-owned artifact was explicitly admitted as foreign. |

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
  non-OK tail-call settlement after a compiler swap; and
- terminal profile changes from inline and computed-head catch/try plus
  variable-trace paths.
