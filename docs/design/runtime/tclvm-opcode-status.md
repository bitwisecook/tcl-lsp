# TCLVM opcode status — C Tcl 9.0 instruction coverage

> **Goal:** the bytecode VM (`rust/tcl-vm`) executes the **same opcode set** as
> the C Tcl 9.0 bytecode engine (`tmp/tcl9.0.4/generic/tclExecute.c`,
> `tclCompile.c` `InstructionDesc`). This is the binary-compatibility checklist:
> all 191 C Tcl 9.0 instructions, ticked off as the VM implements them. Match
> C Tcl semantics exactly (operands, stack effect, error behaviour).

## Legend

- `[x]` — executed by `tcl-vm` (`exec.rs`); a trailing `(note N)` marks a
  documented partial or deliberate divergence, listed under **Notes** below.
- `[~]` — present in the `tcl-bytecode` `Op` enum (the codegen can emit it) but
  **not yet executed** by the VM.
- `[ ]` — **not yet in** the `Op` enum (needs adding to `tcl-bytecode` first,
  matching the C Tcl mnemonic/operands).
Status (auto-countable): **191 executed · 0 enum-only · 0 missing · 191 total**.
Keep this in sync when adding opcodes — update the row and the count.

The coverage was last rebuilt from source (not incremented) against Tcl 9.0.4:
`tclInstructionTable[]` ordering, operand widths, and per-instruction semantics
were audited instruction-by-instruction; the `opcode_family_partition_total`
test (`rust/tcl-bytecode/src/lib.rs`) keeps every enum variant routed to
exactly one mnemonic family and size class, and
`rust/tcl-vm/tests/opcode_c_parity.rs` + `opcode_catch_parity.rs` pin the
C-semantics contracts.

## Instructions (C Tcl 9.0 `InstructionDesc` order)

- [x] `done`
- [x] `push1`
- [x] `push4`
- [x] `pop`
- [x] `dup`
- [x] `strcat`
- [x] `invokeStk1`
- [x] `invokeStk4`
- [x] `evalStk`
- [x] `exprStk`
- [x] `loadScalar1`
- [x] `loadScalar4`
- [x] `loadScalarStk`
- [x] `loadArray1`
- [x] `loadArray4`
- [x] `loadArrayStk`
- [x] `loadStk`
- [x] `storeScalar1`
- [x] `storeScalar4`
- [x] `storeScalarStk`
- [x] `storeArray1`
- [x] `storeArray4`
- [x] `storeArrayStk`
- [x] `storeStk`
- [x] `incrScalar1`
- [x] `incrScalarStk`
- [x] `incrArray1`
- [x] `incrArrayStk`
- [x] `incrStk`
- [x] `incrScalar1Imm`
- [x] `incrScalarStkImm`
- [x] `incrArray1Imm`
- [x] `incrArrayStkImm`
- [x] `incrStkImm`
- [x] `jump1`
- [x] `jump4`
- [x] `jumpTrue1`
- [x] `jumpTrue4`
- [x] `jumpFalse1`
- [x] `jumpFalse4`
- [x] `bitor`
- [x] `bitxor`
- [x] `bitand`
- [x] `eq`
- [x] `neq`
- [x] `lt`
- [x] `gt`
- [x] `le`
- [x] `ge`
- [x] `lshift`
- [x] `rshift`
- [x] `add`
- [x] `sub`
- [x] `mult`
- [x] `div`
- [x] `mod`
- [x] `uplus`
- [x] `uminus`
- [x] `bitnot`
- [x] `not`
- [x] `tryCvtToNumeric`
- [x] `break`
- [x] `continue`
- [x] `beginCatch4` (note 1)
- [x] `endCatch` (note 1)
- [x] `pushResult` (note 1)
- [x] `pushReturnCode` (note 1)
- [x] `streq`
- [x] `strneq`
- [x] `strcmp`
- [x] `strlen`
- [x] `strindex`
- [x] `strmatch`
- [x] `list`
- [x] `listIndex`
- [x] `listLength`
- [x] `appendScalar1`
- [x] `appendScalar4`
- [x] `appendArray1`
- [x] `appendArray4`
- [x] `appendArrayStk`
- [x] `appendStk`
- [x] `lappendScalar1`
- [x] `lappendScalar4`
- [x] `lappendArray1`
- [x] `lappendArray4`
- [x] `lappendArrayStk`
- [x] `lappendStk`
- [x] `lindexMulti`
- [x] `over`
- [x] `lsetList`
- [x] `lsetFlat`
- [x] `returnImm` (note 2)
- [x] `expon`
- [x] `expandStart`
- [x] `expandStkTop`
- [x] `invokeExpanded`
- [x] `listIndexImm`
- [x] `listRangeImm`
- [x] `startCommand` (note 7)
- [x] `listIn`
- [x] `listNotIn`
- [x] `pushReturnOpts` (note 1)
- [x] `returnStk`
- [x] `dictGet`
- [x] `dictSet`
- [x] `dictUnset`
- [x] `dictIncrImm`
- [x] `dictAppend`
- [x] `dictLappend`
- [x] `dictFirst`
- [x] `dictNext`
- [x] `dictUpdateStart`
- [x] `dictUpdateEnd`
- [x] `jumpTable`
- [x] `upvar`
- [x] `nsupvar`
- [x] `variable`
- [x] `syntax` (note 2)
- [x] `reverse`
- [x] `regexp`
- [x] `existScalar`
- [x] `existArray`
- [x] `existArrayStk`
- [x] `existStk`
- [x] `nop`
- [x] `returnCodeBranch`
- [x] `unsetScalar`
- [x] `unsetArray`
- [x] `unsetArrayStk`
- [x] `unsetStk`
- [x] `dictExpand`
- [x] `dictRecombineStk` (note 3)
- [x] `dictRecombineImm` (note 3)
- [x] `dictExists`
- [x] `verifyDict`
- [x] `strmap`
- [x] `strfind`
- [x] `strrfind`
- [x] `strrangeImm`
- [x] `strrange`
- [x] `yield`
- [x] `coroName`
- [x] `tailcall`
- [x] `currentNamespace`
- [x] `infoLevelNumber`
- [x] `infoLevelArgs`
- [x] `resolveCmd`
- [x] `tclooSelf` (note 5)
- [x] `tclooClass`
- [x] `tclooNamespace`
- [x] `tclooIsObject`
- [x] `arrayExistsStk` (note 8)
- [x] `arrayExistsImm` (note 8)
- [x] `arrayMakeStk`
- [x] `arrayMakeImm`
- [x] `invokeReplace`
- [x] `listConcat`
- [x] `expandDrop`
- [x] `foreach_start`
- [x] `foreach_step`
- [x] `foreach_end`
- [x] `lmap_collect` (note 9)
- [x] `strtrim`
- [x] `strtrimLeft`
- [x] `strtrimRight`
- [x] `concatStk`
- [x] `strcaseUpper` (note 10)
- [x] `strcaseLower` (note 10)
- [x] `strcaseTitle` (note 10)
- [x] `strreplace`
- [x] `originCmd`
- [x] `tclooNext` (note 5)
- [x] `tclooNextClass` (note 5)
- [x] `yieldToInvoke`
- [x] `numericType`
- [x] `tryCvtToBoolean`
- [x] `strclass`
- [x] `lappendList`
- [x] `lappendListArray`
- [x] `lappendListArrayStk`
- [x] `lappendListStk`
- [x] `clockRead` (note 11)
- [x] `dictGetDef`
- [x] `strlt`
- [x] `strgt`
- [x] `strle`
- [x] `strge`
- [x] `lreplace4`
- [x] `constImm`
- [x] `constStk`

## Notes

Numbered notes referenced from rows above — the documented partials and
deliberate divergences; everything else matches C per the parity suites.

1. **Exception ranges.** `beginCatch4` opens a *live* in-frame catch range
   only when the instruction carries the out-of-band handler label
   (`Instruction::catch_target` — the analogue of C's
   `ExceptionRange.catchOffset`, kept off the operand so the 4-byte operand
   retains C's range-index meaning and the disassembly stays byte-stable).
   `emit_catch_inline` wires it, making the compiled value-position `catch`'s
   error path executable; a label-less `beginCatch4` is *decorative* — the
   C-faithful reference shape for constructs the VM protects via its
   activation stack instead (`dict for`/`dict map`/`try` epilogues).
   `pushResult`/`pushReturnCode`/`pushReturnOpts` read the absorbed
   completion (result / numeric code / full options dict, with
   `errorInfo`/`errorCode` published exactly as `finish_catch` does).
2. **`returnImm`/`syntax` operand *values* are VM-local — the stack order is
   already C-exact.** C's `INST_RETURN_IMM` reads `OBJ_AT_TOS` as the options
   dict and `OBJ_UNDER_TOS` as the result (`CompileReturnInternal` pushes the
   options *last*), which is exactly what our codegen emits and our arm pops.
   Note C's `returnStk` is the **opposite** order — result on top, options
   under (`tclExecute.c` pops the result first) — and ours matches that too.
   What genuinely diverges is the immediates: a plain top-level `return`
   compiles to `(code 0, level 0)` where C emits `(0, 1)`, and the arm decodes
   but ignores `level`, relying on the options dict for the proc-boundary
   countdown. The VM compensates by treating code 0 as `TCL_RETURN`, whereas
   C's `(0, 0)` — `TclProcessReturn` at level 0 with code OK — *falls through
   to the next instruction*. Codegen and VM are thus self-consistent but both
   diverge from C.

   Realigning is three sites in one change: the codegen encoding, the VM's
   `(code, level)` handling, and the two peephole passes that pattern-match
   the literal `(0, 0)` operand pair (`fold_tail_return_to_done`,
   `strip_unused_start_cmd`) — which would otherwise silently stop firing and
   *change* the emitted bytes. Done together, byte parity with tclsh
   **improves**: the instruction is 9 bytes either way, C also folds a proc's
   plain tail `return` to `done` (in fact more aggressively — any plain
   `return` in a proc with no enclosing catch), and a top-level `return`
   becomes the `returnImm 0 1` tclsh emits. An earlier revision of this note
   claimed the stack order differed and that realigning would break the
   byte-for-byte comparison; both claims were wrong.
3. **`dictRecombineImm`/`dictRecombineStk` ignore the key path** — the
   compiled `dict with` writeback handles a top-level dict variable only, not
   a nested `dict with d k {…}` path (pre-existing in the `Imm` form; the
   `Stk` form mirrors it so the two cannot drift).
4. *(retired — `lindexMulti`/`listIndex` now implement `TclLindexList`/
   `TclLindexFlat` via the runtime `lindex` core.)*
5. **TclOO context test** is "an OO frame is on the VM's call stack" (the
   `next`/`self` commands' existing rule), slightly looser than C's
   `FRAME_IS_METHOD` — a plain proc *called from* a method still counts.
   Opcode and command surfaces agree with each other.
6. *(retired — an unparsable runtime expression now raises C's syntax error.
   `Vm::eval_expr` diagnoses the failure through
   `tcl_syntax::expr::ExprSyntaxError`, which reproduces `ParseExpr`'s messages,
   its `_@_` insert mark, the 25-byte elision window, the
   `TCL PARSE EXPR <detail>` code and the `(parsing expression "…")` `errorInfo`
   frame — `tclCompExpr.c:1397-1471`, pinned against C's `parseExpr-21.*` suite.
   The old fallback had no bearing on the shimmer contract: that document
   describes intrep conversion churn and never relied on an expression
   evaluating to its own text, so the cross-reference here was unfounded. Two
   lowerings that did rely on it were corrected alongside — a `switch` subject
   and a `[…]` expression operand are words, and no longer pass through
   `exprStk`.)*
7. **`startCommand`** is inert (its length/cmd-count operands are carried for
   disassembly parity; the VM needs no interp-epoch recheck).
8. **`arrayExistsImm`/`arrayExistsStk` skip C's `TclCheckArrayTraces`** — the
   VM records `array` trace ops but fires only read/write/unset traces
   anywhere, so the opcodes stay consistent with the VM's `array exists`.
9. **`lmap_collect`** keeps the accumulator in the VM's loop state
   (`ForeachState.accum`) where C uses a compiler temp local; observable
   behaviour matches.
10. **Case-mapping ops** use Unicode *simple* (per-char) mappings like C's
    `Tcl_UniCharToUpper` (`ß` stays `ß`), including C's byte-length guard and
    the Georgian Mtavruli titlecase exception; known residual: `İ` (U+0130)
    stays `İ` where C's table lowercases to `i` (Rust exposes no 1:1 simple
    mapping for it).
11. **`clockRead 0` (clicks) returns microseconds** — the same
    `host.clock().now_micros()` backend the VM's `clock clicks` uses, so
    opcode and command agree (C uses `TclpGetWideClicks`).

- The `Op` enum also carries opcodes that are **not** C Tcl instructions and
  are intentionally outside this checklist: the `irule*` dialect operators
  (all nine now executed — they are emitted for iRules expressions), and five
  extras (`land`/`lor`/`lnot`/`strreverse`/`strrepeat`); `land`/`lor` are
  never emitted and have no dispatch arms (dead), the other three are
  executed VM conveniences.
- Variable opcodes come in `Scalar1/Scalar4/ScalarStk/Array1/Array4/ArrayStk/Stk`
  families — every family member C Tcl emits is covered.
- `foreach_start`/`foreach_step`/`foreach_end` carry the `ForeachInfo` aux
  (loop-var groups) out-of-band on the instruction (`foreach_vars`), and
  `dictUpdateStart`/`dictUpdateEnd` their `DictUpdateInfo` analogue
  (`dict_vars`), keeping operand bytes C-shaped.
