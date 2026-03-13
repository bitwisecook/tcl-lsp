# KCS: Working on Fuzz Findings

How to triage, fix, test, and close differential-fuzzer findings.

## Where findings live

```
tests/fuzz/findings/
  seed_<TIMESTAMP>.json   ← mismatch metadata
  seed_<TIMESTAMP>.tcl    ← Tcl script that triggered the mismatch
```

Each JSON file records mismatches between backends (`vm`, `vm_opt`,
`tclsh(tclsh9.0)`) with fields: `kind`, `description`, `detail_a`,
`detail_b`.

## Triage categories

| Category | Description | Action |
|---|---|---|
| **crash** (ValueError/ZeroDivisionError) | Python exception leaks through VM | Fix: convert to TclError with matching Tcl error message |
| **error_status** (vm error, tclsh ok) | VM rejects valid input | Fix: match tclsh behaviour |
| **error_status** (vm ok, tclsh error) | VM accepts invalid input | Fix: add validation to match tclsh |
| **timeout-all** | All backends timeout equally | Not a bug — mark as fixed with explanation |
| **parse-divergence** | Corrupted input parsed differently | Not a bug — mark as fixed with explanation |
| **output** | Different output values | Fix: trace root cause, match tclsh output |

## Workflow

### 1. Classify the finding

Read the JSON file's `mismatches` array. Each mismatch has:
- `kind`: `crash`, `error_status`, or `output`
- `detail_a` / `detail_b`: error messages or `(ok)` / `TIMEOUT`

Common patterns:
- **Both vm and vm_opt crash, tclsh gives error** → VM bug, needs Tcl-compatible error
- **All three TIMEOUT** → Not a VM bug, expensive input
- **vm ok vs tclsh error** → VM too permissive, needs stricter validation
- **vm error vs tclsh ok** → VM too strict or has different semantics

### 2. Find the minimal reproducer

Extract the relevant expression/command from the `.tcl` file. Test it
in isolation:

```python
from vm.interp import TclInterp
from vm.types import TclError

interp = TclInterp()
try:
    result = interp.eval('expr {91 >> -41}')
    print(f"OK: {result.value}")
except TclError as e:
    print(f"TclError: {e}")
except Exception as e:
    print(f"CRASH: {type(e).__name__}: {e}")
```

### 3. Verify C Tcl 9.0 behaviour

Use the `test-results` skill to find related reference tests:

```bash
python3 .claude/skills/test-results/test_results.py group "expr-*" 9.0
python3 .claude/skills/test-results/test_results.py source expr-23.24 9.0
```

Search Tcl test sources for exact error messages:

```bash
grep -rn "negative shift" tmp/tcl9.0.3/tests/
```

### 4. Fix — prefer early pipeline stages and LSP diagnostics

**General principle**: fix as early in the pipeline as possible. A bug
caught at lowering time or via a diagnostic is better than one that only
surfaces at VM runtime, because the user gets immediate editor feedback.

**Fix priority** (earliest first):

1. **Lowering / IR** (`core/compiler/lowering.py`) — reject malformed
   structures by emitting an `IRBarrier` so the bytecode compiler never
   sees them. This also enables a diagnostic (see step 4b).
2. **Compiler checks / diagnostics** (`core/compiler/compiler_checks.py`) —
   emit an `E0xx` diagnostic for the `IRBarrier` so the LSP shows an
   error squiggle in the editor *before* the user runs the code.
3. **Interpreter command handler** (`vm/commands/`) — add runtime
   validation for the same pattern so the interpreter path also rejects it.
4. **Bytecode VM** (`vm/machine.py`) — catch Python exceptions and
   convert to `TclError`.

Not every fix needs all four layers, but always consider whether earlier
detection is possible. If a structural error can be detected statically
(e.g. `if-else-elseif`), it **must** also produce a diagnostic.

**Example: if-else-elseif (seed 1772823178)**

The pattern `if {cond} {body} else {body} elseif ...` is statically
detectable. Fixes were applied at three layers:
- **Lowering**: `_lower_if` emits `IRBarrier(reason='extra words after "else" clause')`
- **Compiler check**: `_arity_checks` detects the barrier and emits diagnostic `E004`
- **Runtime**: `_cmd_if` pre-validates args before evaluating any branch

Key files:
- `vm/machine.py` — bytecode VM (arithmetic, bitwise, incr opcodes)
- `vm/interp.py` — AST-based expression evaluator
- `vm/commands/control.py` — `for`, `while`, `foreach` commands
- `vm/commands/math_cmds.py` — `incr` command
- `vm/commands/string_cmds.py` — `string` subcommands
- `core/compiler/lowering.py` — IR lowering (structural validation)
- `core/compiler/compiler_checks.py` — IR-based diagnostics

Common fix patterns:

**Python exception → TclError**: Catch the Python exception and raise
TclError with the matching Tcl error message:
```python
if b < 0:
    raise TclError("negative shift argument")
```

**Missing opcode**: Add a `case Op.NEW_OP:` handler in `machine.py`.

**Wrong return value**: Tcl loop commands (`foreach`, `while`, `for`)
always return `""`. Ensure `return TclResult()` not `return result`.

**Too-permissive parsing**: Use `_parse_int_strict()` instead of
`_parse_int()` when the Tcl command doesn't accept boolean literals
(e.g., `incr`).

**Structural command error → diagnostic**: When a command has a
statically detectable structural error, emit an `IRBarrier` in lowering
and detect it in `compiler_checks.py` to produce an LSP diagnostic:
```python
# In _arity_checks._check_statement:
if isinstance(stmt, IRBarrier) and stmt.command == "if":
    if "extra words" in stmt.reason:
        diagnostics.append(Diagnostic(..., code="E004"))
```

### 5. Write regression tests

Add tests to `tests/test_fuzz_findings.py`. Name test classes and
methods after the bug category, and reference the seed number:

```python
class TestNegativeShift:
    def test_seed_1772822371(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="negative shift argument"):
            interp.eval("expr {(-39 >> (98 ? -72 : -76))}")

    def test_seed_1772822371_full(self) -> None:
        source = (FINDINGS_DIR / "seed_1772822371.tcl").read_text()
        status, detail = _run(source)
        assert status == "error"
```

Include both:
- **Minimal reproducer** tests (isolated expression/command)
- **Full script** tests (run the entire `.tcl` file)

### 6. Mark the finding as fixed

Add a `"fixed"` key (and optionally `"fix"` and `"category"`) to the
JSON file:

```json
{
  "seed": 1772822371,
  "mismatches": [...],
  "fixed": true,
  "fix": "negative shift argument check in _bitwise_binary",
  "category": "negative-shift"
}
```

### 7. Run existing tests

Always verify no regressions:

```bash
pytest tests/test_fuzz_findings.py -v
pytest tests/test_vm_basic.py tests/test_vm_control.py tests/test_optimiser_vm_equivalence.py -x -q
```

### 8. Commit

Group related fixes in a single commit. Reference the seed numbers.

## Common root causes found by the fuzzer

| Root cause | Seeds | Fix |
|---|---|---|
| Negative shift not checked | 330, 371, 398, 459, 460, 476, 3034, 3085, 3100, 3160, 3167, 3182, 3191 | `_bitwise_binary`: check `b < 0` before `<<`/`>>` |
| Zero ** negative not checked | 472, 3130 | `_arith_binary`: check `a == 0 and b < 0` |
| Float overflow in `**` | 3123 | `_arith_binary`: catch `OverflowError`, return `Inf`/`-Inf` |
| `if-else-elseif` accepted | 3178 | Lowering barrier + E004 diagnostic + `_cmd_if` pre-validation |
| Loop return value wrong | 499 | `foreach`/`while`/`for`: `return TclResult()` |
| `incr` accepts booleans | 419, 3091 | Use `_parse_int_strict` (no boolean coercion) |
| `STR_RFIND` opcode missing | 489 | Add `case Op.STR_RFIND:` handler |
| Expensive input (all timeout) | 367, 379, 404, 457, 501, 3045, 3119, 3157, 3161, 3194 | Not a bug |
| Corrupted input parse diff | 388, 395 | Not a bug |
| VM timeout, tclsh errors | 3184 | Loop return value fix prevents infinite loop |

*Seeds listed as short suffixes (e.g. 3034 = 1772823034).*

## JSON schema reference

```json
{
  "seed": 1772822330,
  "mismatches": [
    {
      "kind": "crash | error_status | output",
      "description": "human-readable summary",
      "backend_a": "vm | vm_opt | tclsh(tclsh9.0)",
      "backend_b": "vm | vm_opt | tclsh(tclsh9.0)",
      "detail_a": "error message | (ok) | TIMEOUT | CRASH: ...",
      "detail_b": "error message | (ok) | TIMEOUT | CRASH: ..."
    }
  ],
  "fixed": true,
  "fix": "short description of the fix",
  "category": "negative-shift | zero-negative-power | loop-return-value | ..."
}
```
