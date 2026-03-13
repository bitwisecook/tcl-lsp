# KCS: Namespace resolution

## Symptom

A contributor needs to understand how qualified names (`::foo::bar`) are
normalised and resolved through the compilation pipeline, or is debugging
why a procedure call is not matched to its definition.

## Context

Tcl uses `::` as the namespace separator with `::` as the global namespace.
`normalise_qualified_name()` canonicalises names, and lowering propagates
namespace context so that `proc` definitions inside `namespace eval` receive
fully qualified names.  Interprocedural analysis uses namespace-aware call
resolution.

Source: [`core/common/naming.py`](../../../core/common/naming.py),
[`core/compiler/lowering.py`](../../../core/compiler/lowering.py),
[`core/compiler/interprocedural.py`](../../../core/compiler/interprocedural.py)

## Content

### `normalise_qualified_name()`

```python
normalise_qualified_name("helper")          → "::helper"
normalise_qualified_name("::helper")        → "::helper"
normalise_qualified_name("mylib::helper")   → "::mylib::helper"
normalise_qualified_name("::::foo::::bar")  → "::foo::bar"
```

Rules: strip trailing `::`, collapse multiple `::` runs, ensure leading `::`.

### Namespace context propagation during lowering

`_lower_command()` carries a `namespace` parameter:

1. Top-level: `namespace="::"`
2. `namespace eval mylib { ... }`:
   - `_join_namespace("::", "mylib")` → `"::mylib"`
   - Body lowered with `namespace="::mylib"`
3. `proc helper` inside `::mylib`:
   - `_qualify_proc_name("::mylib", "helper")` → `"::mylib::helper"`

### Call resolution in interprocedural analysis

`resolve_internal_call("helper", "::mylib::compute", known_procs)`:

1. Extract namespace parts from caller: `["mylib"]`
2. Try `::mylib::helper` → found → return it
3. If not found, try `::helper` (global namespace)
4. Walk up the hierarchy until found or return `None`

### Resulting IR module

```python
IRModule(
    procedures={
        "::mylib::helper": IRProcedure(name="helper", ...),
        "::mylib::compute": IRProcedure(name="compute", ...),
    },
)
```

All procedure names in the IR module are fully qualified.

## Decision rule

- Always pass names through `normalise_qualified_name()` before using them
  as dictionary keys or comparison targets.
- If a procedure call fails to resolve, check that the caller's namespace
  context was propagated correctly through lowering.
- `normalise_var_name()` is for variables (handles `::` prefix only);
  `normalise_qualified_name()` is for procedures and commands.

## Related docs

- [Example 26 in walkthroughs](../../example-script-walkthroughs.md#example-26-namespace-resolution)
- [kcs-compiler-pipeline-overview.md](kcs-compiler-pipeline-overview.md)
- [kcs-interprocedural-analysis.md](kcs-interprocedural-analysis.md)
