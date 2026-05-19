# Contract: WASM explorer disassembly view

## Purpose

The compiler explorer renders the output of `wasm_codegen_module` as a
per-instruction interactive disassembly.  The JSON shape that
[`explorer/serialise.py`](../../../explorer/serialise.py) produces and
that [`explorer/static/explorer-core.js`](../../../explorer/static/explorer-core.js)
consumes is fixed by this contract.  Both the standalone web panel
(`explorer/static/index.html`) and the VS Code webview
(`editors/vscode/src/compilerExplorerHtml.ts`) read the same shape.

## Producer

- [`compiler/codegen/wasm/_ir.py`](../../../compiler/codegen/wasm/_ir.py)
  — `WasmModule.to_explorer_json()` returns a list of function entries
  (plus a synthetic module header).  Each instruction carries a decoded
  target (`call` → function name, `br` / `br_if` → matching structural
  open/close), a source range, an indent level, and an explorer label.
- [`explorer/serialise.py`](../../../explorer/serialise.py) —
  `_serialise_wasm` calls `to_explorer_json()`, attaches a WAT text
  snippet per entry for legacy consumers, and returns the list as
  `data.wasm` / `data.wasmOptimised`.

## Module header entry

The first entry in each list is always the synthetic module header.
`instrCount` is fixed at 0 so tab badge totals (summed over entries)
don't double-count body instructions; the real total is available as
`totalInstrCount`.

```jsonc
{
  "name": "(module)",
  "kind": "module",
  "funcIdx": null,
  "params": [],
  "results": [],
  "locals": [],
  "sourceRange": null,
  "instrCount": 0,
  "totalInstrCount": 32,
  "instructions": [],
  "imports": [
    {"module": "tcl", "name": "tcl_obj_get_int", "typeIdx": 0, "funcIdx": 0}
  ],
  "types": [{"index": 0, "params": ["i32"], "results": ["i64"]}],
  "dataSegments": [{"offset": 0, "size": 8}],
  "text": "(module (type …) (import …) …)"
}
```

## Function entry

One entry per function in the module, in declaration order (top-level
first, then user procs).

```jsonc
{
  "name": "::greet",
  "kind": "proc",            // "top" | "proc" | "method"
  "funcIdx": 23,             // absolute WASM function index (imports + defined)
  "exported": true,
  "params": [{"name": "$name", "type": "i32"}],
  "results": ["i32"],
  "locals": [{"name": "$msg", "type": "i32"}],
  "sourceRange": {
    "startLine": 0, "startCol": 0, "startOffset": 0,
    "endLine": 2, "endCol": 15, "endOffset": 58
  },
  "instrCount": 9,
  "instructions": [...],
  "text": "(func $::greet …)"
}
```

## Instruction entry

Each entry in `instructions` carries a stable `idx` — the same index
used by `br_target.targetIdx` for cross-navigation.

```jsonc
{
  "idx": 33,                   // stable index within the function body
  "indent": 2,                 // control-flow nesting depth (for display)
  "op": "br_if",               // mnemonic
  "opcode": 13,                // raw WASM opcode
  "operandText": "1",          // decoded operand as a string
  "fullText": "br_if 1",       // convenience: "op operand"
  "range": {                   // originating Tcl source range (nullable)
    "startLine": 6, "startCol": 0, "startOffset": 87,
    "endLine": 6, "endCol": 36, "endOffset": 123
  },
  "label": "foreach break",    // explorer hint on structural ops
  "callTarget": null,          // set when op == "call"
  "branchTarget": {            // set when op in {"br", "br_if"}
    "depth": 1,
    "targetIdx": 69,           // idx of the matching end/loop_header
    "kind": "block_end",       // "block_end" | "loop_header" | "if_end"
    "label": "foreach break"   // from the matching open's label
  },
  "blockLabel": null,          // "$L28" — set on block/loop/if opens
  "blockKind": null,           // "block" | "loop" | "if" — set on opens
  "localIndex": null           // decoded LEB128 local index on local.*
}
```

### `callTarget` shape

```jsonc
{
  "kind": "import",       // "import" | "top" | "proc" | "method" | "unknown"
  "name": "tcl_list_index",
  "module": "tcl",        // null for non-imports
  "funcIdx": 10,          // absolute WASM function index
  "defIdx": null          // 0-based index into WasmModule.functions, or null for imports
}
```

The frontend looks up `defIdx` in the function-entry list to navigate
to the matching disassembly block (and, via `sourceRange`, to the
source).  Imports have no disassembly to jump to.

### `branchTarget.kind`

| Kind | Meaning |
|------|---------|
| `loop_header` | `br` / `br_if` back-edge to a `loop` open |
| `block_end` | forward exit to the `end` of a `block` |
| `if_end` | forward exit to the `end` of an `if` |
| `function_return` | depth exceeds the structural stack (branch out of the function) |

## Emitter contract

`_WasmEmitter._emit` stamps `self._current_range` onto every emitted
instruction.  `_record_stmt_context` (called at the top of
`_emit_stmt` and at every CFG block entry via the terminator-range
stamp) keeps that range up to date.  Callers that open a structural
op (`block` / `loop` / `if`) may pass `label=` to `_emit` to attach a
human-readable tag (e.g. `foreach`, `if`, `catch body`) that the
explorer surfaces in both the open's inline label and in the target
hint on every `br` / `br_if` that lands on its matching close.

## Versioning

The shape above is additive.  Consumers that don't understand a new
field must ignore it; producers must never repurpose an existing field
name with a different type.  When fields change meaning, rename them.

## Related

- [KCS: feature — Compiler Explorer](../../kcs/features/kcs-feature-compiler-explorer.md)
- [Codegen module map](../compiler/codegen-module-map.md)
- [WASM runtime primitives](../compiler/wasm-runtime-primitives.md)
