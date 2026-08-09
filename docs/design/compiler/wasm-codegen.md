# WASM codegen pipeline

The Rust WASM backend emits Tcl-object operations from the compiler's lowered
program. Unsupported statements retain an exact source-span fallback to the
runtime interpreter.

## Entry points

`codegen::wasm::wasm_codegen_compilation_unit` is the analysis-aware entry
point used by `tcl compwasm --backend tree-walker`. It consumes the existing
`CompilationUnit`, including its lowered IR, CFG, SSA, SCCP results, type
lattices, and original CST-derived source spans.

`wasm_codegen_module` remains available for callers that only have an IR
module. It emits the structured control-flow and source-evaluation fallback,
but does not attempt direct calls because it has no binding proof.

## Direct-emission proof

The backend specialises a statement only when all required compiler facts
agree:

1. Lowering produced a typed IR statement or expression AST the emitter
   supports.
2. The flow-sensitive command-binding lattice proves that a builtin or user
   procedure name still denotes the expected command at that statement.
3. The whole-module command-mutation summary proves no procedure body can
   rename or alias that binding later.
4. A direct arithmetic procedure has a numeric return in the type lattice and
   a supported expression tree.
5. The command's `CommandSpec.wasm_codegen_hook` selects the runtime operation.

If any proof is absent, the structured walk passes the statement's original
source span to `tcl_eval_code`. This keeps dynamic Tcl semantics as the
conservative boundary.

## Tcl-object stack and variables

Generated values are `i32` pointers to owned `TclObj` values in shared linear
memory. Procedure parameters are WebAssembly parameters of that same type.

Compiled procedure locals have two ports onto one runtime variable cell:

- an indexed slot used by generated `local_get` and `local_set` operations;
- the ordinary Tcl name used by traces, `upvar`, and interpreted fallback.

The procedure prologue binds each slot index to its Tcl-visible name in the
normal call frame. This deliberately preserves Tcl semantics before later
escape and representation passes prove that a variable can become a plain
WebAssembly local.

## Current direct tier

The initial direct tier covers:

- literal top-level `set` assignments;
- lowered procedure registration without evaluating the `proc` command;
- fixed-arity procedures whose body is a supported numeric return expression;
- variable reads through indexed procedure slots or named top-level cells;
- Tcl numeric-tower addition;
- binding-proven direct user-procedure calls in command substitution;
- the one-argument stdout form of registry-stamped `puts`.

For example, the procedure in:

```tcl
proc add {b c} {
    return [expr {$b + $c}]
}

set e 2
set f 4
puts [add $e $f]
```

is emitted as an `(i32, i32) -> i32` WebAssembly function. The top-level
function registers its source metadata, stores `e` and `f`, loads both values,
calls the generated `::add` function, and passes its result to the runtime
`puts` primitive. None of those statements calls `tcl_eval_code`.

## Runtime ownership contract

The codegen ABI in `runtime/rust/src/codegen_abi.rs` uses one owned reference
for every generated operand-stack value:

| Operation | Ownership |
|---|---|
| `tcl_value_new_string` | returns `+1` |
| variable load | returns a new `+1` beside the cell's reference |
| variable store/bind | consumes the operand `+1`; the cell retains its own |
| arithmetic add | consumes both operands and returns `+1` |
| direct procedure call | transfers argument references to the callee |
| procedure return | transfers one result reference to the caller |
| `tcl_codegen_puts` | consumes its value |

Procedure frame push/pop uses the runtime's ordinary `FrameStack`; popping a
frame releases its stored variable references.

## Module layout

Runtime functions are imported first. `::top` is the first defined function,
followed by user procedures in qualified-name order, so direct call indices are
deterministic. A relocated module places its constant pool at
`RESERVED_DATA_BASE`, inside the memory window reserved by the Rust runtime.

The WAT renderer and binary encoder both consume the same `WasmModule` IR in
`rust/tcl-compiler/src/codegen/wasm/ir.rs`.
