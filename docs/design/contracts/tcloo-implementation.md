# KCS: TclOO implementation

## Summary

The TclOO subsystem provides class hierarchy analysis for the LSP and runtime
execution in the bytecode VM.  It covers `oo::class create`, `oo::define`,
`oo::objdefine`, constructors, destructors, methods, mixins, filters, private
variables/methods (TIP 500), properties (TIP 558), and configurable support.

## Architecture

### LSP analysis layer (`analyser/`)

The analyser (`analyser.py`) recognises `oo::class create` and `oo::define`
blocks during static analysis, building `ClassDef` entries in the semantic
model.  These feed:

- **Hover** (`server/features/hover.py`) -- shows class hierarchy, method
  signatures, and inherited methods.
- **Go-to-definition** (`server/features/definition.py`) -- jumps to method
  bodies and class definitions.
- **Completion** (`server/features/completion.py`) -- suggests methods in `my`
  and `self` contexts.
- **Type hierarchy** (`server/features/type_hierarchy.py`) -- supertypes and
  subtypes for `oo::class` definitions.

### MRO algorithm (`analyser/mro.py`)

Method resolution order uses a linearisation matching C Tcl's algorithm
(exposed as `tcloo_linearise`).  The MRO considers superclasses and mixins
and caches results per class, invalidating when the class hierarchy changes.

### VM runtime layer (`tooling/vm/oo.py`, `tooling/vm/commands/oo_cmds.py`)

The `OORuntime` class manages the object/class registry at runtime:

- **Object lifecycle** -- creation, destruction, namespace allocation
  (`::oo::Obj<N>` unique namespaces).
- **Method dispatch** -- walks MRO to find methods, applies filter chains,
  handles `next`/`nextto` for chain dispatch.
- **Variable binding** -- `my variable`, `my varname`, private variable
  mangling using creation IDs (TIP 500).
- **Introspection** -- `info object`, `info class` subcommands for methods,
  mixins, filters, variables, superclasses, and instances.

### Command handlers (`tooling/vm/commands/oo_cmds.py`)

Registers `oo::class`, `oo::define`, `oo::objdefine`, and all definition
subcommands (method, constructor, destructor, superclass, mixin, filter,
variable, forward, export, unexport, deletemethod, renamemethod, self,
abstract, private, property, readableproperties, writableproperties,
definitionnamespace).

Class name resolution during `oo::define` body evaluation uses
`_defining_caller_ns` to resolve relative names in the namespace where
`oo::define` was invoked, not the `::oo::define` evaluation namespace.

## Test conformance

Native Tcl 9.0.3 test suite results:

| Test file | Passed | Skipped | Failed | Conformance |
|-----------|--------|---------|--------|-------------|
| oo.test | 330 | 16 | 42 | 85% |
| ooNext2.test | 57 | 5 | 0 | 100% (of non-skipped) |

Remaining failures require VM features not yet implemented: sub-interpreter
isolation (`interp create`), `tailcall`, `rename` interaction with OO objects,
`coroutine`, ensemble error rewriting, `bgerror`/`update`, and error trace
variable preservation.

## Key files

| File | Role |
|------|------|
| `tooling/vm/oo.py` | Core OO runtime (object/class registry, dispatch, MRO) |
| `tooling/vm/commands/oo_cmds.py` | OO command handlers and define body parsing |
| `tooling/vm/commands/info_cmds.py` | `info object`/`info class` introspection |
| `tooling/vm/scope.py` | CallFrame with OO variable binding slots |
| `analyser/mro.py` | MRO linearisation algorithm |
| `analyser/analyser.py` | Static OO analysis (class/method extraction) |
| `tests/test_vm_oo_test.py` | Native test runner with known-failure tracking |
