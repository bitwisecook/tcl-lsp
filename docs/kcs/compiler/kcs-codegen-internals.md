# KCS: Codegen internals — LVT, linearisation, labels, peephole

## Symptom

A contributor needs to understand how the bytecode emitter transforms CFG/SSA
into instruction sequences, or is debugging a mismatch between our output and
tclsh's disassembly.

## Context

The codegen phase takes a `CompilationUnit` with CFG/SSA and produces
`FunctionAsm` objects containing instruction lists, literal tables, and local
variable tables (LVT).  The emitter performs block linearisation (RPO),
bottom-tested loop reordering, label-based jump emission, iterative jump
size optimisation, and peephole passes — all designed to match tclsh's
bytecode output.

Source: [`core/compiler/codegen/_emitter.py`](../../../core/compiler/codegen/_emitter.py),
[`core/compiler/codegen/layout.py`](../../../core/compiler/codegen/layout.py),
[`core/compiler/codegen/_peephole.py`](../../../core/compiler/codegen/_peephole.py),
[`core/compiler/codegen/_types.py`](../../../core/compiler/codegen/_types.py)

## Content

### Step 1 — LVT allocation

The `_Emitter` constructor creates a `LocalVarTable` from the parameter list.
Inside a `proc`, all variable accesses use LVT-indexed instructions
(`loadScalar1 %v0`) instead of name-based stack operations (`loadStk`).

```python
LocalVarTable(params=("n",))
# LVT slots: %v0 = "n"
```

### Step 2 — Block linearisation

`_linearise()` performs a DFS from the entry block to produce a reverse
post-order (RPO).  This determines instruction layout such that the common
(fall-through) path appears immediately after each branch.

For loops, `_reorder_bottom_tested()` detects back-edges and moves the loop
body before the header, producing tclsh's condition-at-bottom layout:

```
Before (top-tested):   header → body → jump header
After  (bottom-tested): jump header → body → header (jumpTrue body)
```

### Step 3 — Instruction emission with labels

The emitter walks linearised blocks, placing labels and emitting instructions:

```
_place_label("entry_1")        → label at instruction 0
  emit(LOAD_SCALAR1, %v0)     # load n
  emit(PUSH1, lit("0"))       # push "0"
  emit(LT)                    # n < 0
  emit(JUMP_FALSE4, "L_else") # → else branch

_place_label("if_then_3")
  emit(LOAD_SCALAR1, %v0)
  emit(UMINUS)
  emit(JUMP4, "L_end")

_place_label("L_else")
  emit(LOAD_SCALAR1, %v0)

_place_label("L_end")
  emit(DONE)
```

### Step 4 — Jump size optimisation

`optimise_jumps()` in `layout.py` iterates up to 10 times, replacing 4-byte
jumps with 1-byte jumps when the relative offset fits in [-128, 127].
Shortening jumps changes sizes which changes offsets — hence the iterative
approach.

### Step 5 — Label resolution

`resolve_layout()` assigns concrete byte offsets and patches jump operands
from label names to relative byte offsets.

### Step 6 — Peephole passes

1. **`_remove_trailing_pop()`**: Strip `pop; done` → `done` (last value
   stays on stack for return).
2. **`_fold_const_push_pop_nops()`**: Dead `push; pop` pairs become
   `nop; nop; nop` — matching tclsh's 3-nop pattern for folded constants.
3. **`_dedup_push_literals()`**: Deduplicate literal slots after nop-folding.

### Worked example — `proc abs {n}`

```
  LVT:  %v0="n"
  Literals:  0="0"

  (0)  loadScalar1 %v0  # load n
  (2)  push1 0          # "0"
  (4)  lt               # n < 0 ?
  (5)  jumpFalse1 +5    # jump to pc 10
  (7)  loadScalar1 %v0  # load n (then-body)
  (9)  uminus           # negate
  (10) jump1 +3         # jump to pc 13
  (12) loadScalar1 %v0  # load n (else-body)
  (14) done
```

## Decision rule

- If bytecode does not match tclsh, check linearisation order first (RPO may
  differ if block ordering changed), then peephole passes.
- LVT slots are allocated in parameter order, then in first-use order for
  local variables.
- The 4-byte → 1-byte jump optimisation is the most common source of size
  differences vs a naïve emitter.

## Related docs

- [Example 27 in walkthroughs](../../example-script-walkthroughs.md#example-27-codegen-internals--labels-lvt-linearisation-peephole)
- [GLOSSARY.md — LVT](../../GLOSSARY.md#lvt)
- [kcs-codegen-module-map.md](kcs-codegen-module-map.md)
- [kcs-bytecode-boundary.md](kcs-bytecode-boundary.md)
