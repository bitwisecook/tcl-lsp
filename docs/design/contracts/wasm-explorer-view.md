# Contract: WASM explorer disassembly view

## Purpose

The compiler explorer renders the output of the canonical `compile_wasm`
pipeline as a
per-instruction interactive disassembly.  The JSON shape that
[`rust/tcl-explorer`](../../../rust/tcl-explorer) produces and
that [`rust/tcl-cli/gui/explorer-core.js`](../../../rust/tcl-cli/gui/explorer-core.js)
consumes is fixed by this contract.  Both the standalone web GUI
([`rust/tcl-cli/gui/index.html`](../../../rust/tcl-cli/gui/index.html), served
by `tcl explore --serve` and published to GitHub Pages) and the VS Code webview
([`editors/vscode/src/compilerExplorerHtml.ts`](../../../editors/vscode/src/compilerExplorerHtml.ts))
read the same shape.

## Producer

- [`rust/tcl-explorer/src/wasm_explorer.rs`](../../../rust/tcl-explorer/src/wasm_explorer.rs)
  — `wasm_to_explorer_json(&WasmModule, &LineIndex, source)` returns a list
  of function entries (plus a synthetic module header).  Each instruction
  carries a decoded target (`call` → function name, `br` / `br_if` → matching
  structural open/close), a source range, an indent level, and an explorer
  label.
- [`rust/tcl-explorer/src/serialise.rs`](../../../rust/tcl-explorer/src/serialise.rs)
  — `serialise_wasm` calls the canonical compiler, attaches the typed
  `codegenPlan` evidence and full WAT `text` on the module header, and returns
  the list as
  `data.wasm` / `data.wasmOptimised`.
- [`rust/tcl-explorer-wasm`](../../../rust/tcl-explorer-wasm) — the
  `wasm-bindgen` facade the browser worker calls (`compile`, plus `meta` for
  the dialect list, which needs no compile).

## Module header entry

The first entry in each list is always the synthetic module header.
`instrCount` is fixed at 0 so tab badge totals (summed over entries)
don't double-count body instructions; the real total is available as
`totalInstrCount`, and `functionCount` is the number of defined functions
(i.e. the entry count minus this header).

`types` is the module's type section as the binary/WAT serialiser emits it —
import signatures first (interned when the import is registered), then any
further defined-function signature.  Producers must compute it with
`WasmModule::type_section()` rather than reading the private `types` field:
defined-function signatures are only interned when the module is actually
serialised, so a not-yet-emitted module under-reports otherwise.
Every `imports[].typeIdx` indexes into this list.

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
  "functionCount": 4,
  "instructions": [],
  "imports": [
    {"module": "tcl", "name": "tcl_obj_get_int", "typeIdx": 0, "funcIdx": 0}
  ],
  "types": [{"index": 0, "params": ["i32"], "results": ["i64"]}],
  "dataSegments": [{"offset": 0, "size": 8}],
  "codegenPlan": {
    "kind": "generic-invoke", // or "compatibility"
    "operation": "intrinsic", // invoke | intrinsic | structured-lowering
    "compatibility": null      // typed reason object after a decline
  },
  "text": "(module (type …) (import …) …)"
}
```

`codegenPlan` is durable compiler evidence, not display inference. A
compatibility plan carries stable `kind` and `detailKind` fields explaining
why executable semantic selection declined. It never contains Rust `Debug`
output.

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

The canonical `compile_wasm` pipeline produces a `WasmModule` whose
instructions retain source ranges and optional structural labels. Both the
semantic executable-IR emitter and the private typed compatibility plan write
the same target IR; Explorer never invokes an emitter of its own.

`wasm_to_explorer_json` derives block nesting and branch targets from that
shared instruction stream. Structural operations (`block`, `loop`, and `if`)
may carry a human-readable label such as `foreach`, `if`, or `catch body`.
Explorer surfaces the label on the opening operation and on every `br` or
`br_if` whose decoded target reaches the matching close.

## Versioning

The shape above is additive.  Consumers that don't understand a new
field must ignore it; producers must never repurpose an existing field
name with a different type.  When fields change meaning, rename them.

## View descriptors and World SSA

`meta.views` is the ordered Explorer view catalogue. Every entry carries an
`id`, `label`, `payload`, `group`, and `renderKind`. Consumers use it to reconcile their
tabs: a view without a bespoke renderer remains visible through the shared
structured fallback rather than being silently omitted.

`worldSsa` is an array of per-function compiler semantic sidecars. Each entry
always has an `availability` object with a stable `kind`, `hasExecutableIr`,
and optional typed `reasonKind`. When the graph is available, `locations` keep
the precise domain, interpreter, namespace, subject, and external-resource
identity; `operations` carry state versions plus node, CFG, or edge sites.
`phi` operations include explicit predecessor versions and `includesInitial`.
Resolved invocation entries retain registry-projected transitions, their
completion commit policy, abrupt-edge transfer, and the typed proof inputs
(result stability and dispatch dependencies) needed to explain a GVN reuse or
abstention. A decline is data, not an empty proof: consumers must show it and
must not infer that no mutable world state exists.

## Consumer resilience

Producer and consumer ship in the same binary but are versioned by hand, and
a published GUI can outlive the payload it was built against (GitHub Pages
serves a cached `explorer-core.js`).  Two rules keep a mismatch survivable —
both were learned from issues #1182 / #1183, where a module header that had
lost its `types` array made `renderWasmModuleHeader` throw, blanked the WASM
tab, and left the compile spinner throbbing forever:

1. **Renderers treat every list as optional.**  Read `entry.foo || []`, never
   `entry.foo.length`.  A field the producer has not caught up with must
   degrade to "0 of those", not to a `TypeError`.
2. **Rendering is isolated per pane.**  Both consumers drive their render
   pipeline through `runRenderSteps` (in `explorer-core.js`): a step that
   throws reports the error inside its own pane, the remaining panes still
   render, and `renderAll` itself never throws — so the caller's spinner and
   status light always settle, whatever a renderer did.

Limitation: this makes a contract drift *visible and non-fatal*, not
harmless.  A pane fed a payload missing data it genuinely needs will render
that data as absent (or show the error in place of its content); only the
producer-side tests
(`rust/tcl-explorer/src/wasm_explorer.rs::module_header_carries_every_contract_field`)
assert the payload is complete.

## Tests

- `rust/tcl-explorer/src/wasm_explorer.rs` (unit) — the module header carries
  every contract field, and `types` covers imports plus defined functions.
- `rust/tcl-compiler/src/codegen/wasm/ir.rs` (unit) — `type_section()`
  predicts the type table `to_bytes` emits.
- `rust/tcl-cli/tests/explorer_gui.rs` (integration) — drives the shipped GUI
  in headless Chromium against a real payload and asserts the WASM tab
  renders, the spinner stops, and no pane errors.  Skips with a printed
  reason when node/Playwright are unavailable; set `TCL_EXPLORER_GUI_TEST=1`
  to make a missing toolchain a failure instead.
- `editors/vscode/src/test/compilerExplorerWebview.test.ts` — the generated
  webview HTML keeps the isolation wiring and the optional-list guards.

## Related

- [KCS: feature — Compiler Explorer](../../kcs/features/kcs-feature-compiler-explorer.md)
- [Codegen module map](../compiler/codegen-module-map.md)
- [WASM runtime primitives](../compiler/wasm-runtime-primitives.md)
