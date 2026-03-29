# KCS: TclOO implementation

## Summary

The TclOO subsystem provides class hierarchy analysis for the LSP and runtime
execution in the bytecode VM.  It covers `oo::class create`, `oo::define`,
`oo::objdefine`, constructors, destructors, methods, mixins, filters, private
variables/methods (TIP 500), properties (TIP 558), and configurable support.

## Architecture

### LSP analysis layer (`core/analysis/`)

The analyser (`analyser.py`) recognises `oo::class create` and `oo::define`
blocks during static analysis, building `ClassDef` entries in the semantic
model.  These feed:

- **Hover** (`lsp/features/hover.py`) -- shows class hierarchy, method
  signatures, and inherited methods.
- **Go-to-definition** (`lsp/features/definition.py`) -- jumps to method
  bodies and class definitions.
- **Completion** (`lsp/features/completion.py`) -- suggests methods in `my`
  and `self` contexts.
- **Type hierarchy** (`lsp/features/type_hierarchy.py`) -- supertypes and
  subtypes for `oo::class` definitions.

### MRO algorithm (`core/analysis/mro.py`)

Method resolution order uses a linearisation matching C Tcl's algorithm
(exposed as `tcloo_linearise`).  The MRO considers superclasses and mixins
and caches results per class, invalidating when the class hierarchy changes.

### VM runtime layer (`vm/oo.py`, `vm/commands/oo_cmds.py`)

The `OORuntime` class manages the object/class registry at runtime:

- **Object lifecycle** -- creation, destruction, namespace allocation
  (`::oo::Obj<N>` unique namespaces).
- **Method dispatch** -- walks MRO to find methods, applies filter chains,
  handles `next`/`nextto` for chain dispatch.
- **Variable binding** -- `my variable`, `my varname`, private variable
  mangling using creation IDs (TIP 500).
- **Introspection** -- `info object`, `info class` subcommands for methods,
  mixins, filters, variables, superclasses, and instances.

### Command handlers (`vm/commands/oo_cmds.py`)

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
| `vm/oo.py` | Core OO runtime (object/class registry, dispatch, MRO) |
| `vm/commands/oo_cmds.py` | OO command handlers and define body parsing |
| `vm/commands/info_cmds.py` | `info object`/`info class` introspection |
| `vm/scope.py` | CallFrame with OO variable binding slots |
| `core/analysis/mro.py` | MRO linearisation algorithm |
| `core/analysis/analyser.py` | Static OO analysis (class/method extraction) |
| `tests/test_vm_oo_test.py` | Native test runner with known-failure tracking |
