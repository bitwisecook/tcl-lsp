# VM opcode coverage — the C Tcl 9.0 instruction set

Binary compatibility means the bytecode VM (`rust/tcl-vm`) executes the **same
opcode set** as the C Tcl 9.0 bytecode engine
(`tmp/tcl9.0.4/generic/tclExecute.c`, `tclCompile.c`'s `InstructionDesc`), with
matching operands, stack effect, and error behaviour. This is the inventory of
where each of the 191 instructions currently stands.

Coverage: **191 executed · 0 enum-only · 0 absent · 191 total**. It is
maintained by hand — adding an opcode means updating its row and the count in
the same change. The rows are checked against the `Op` enum in
`rust/tcl-bytecode/src/lib.rs` and the dispatch arms in
`rust/tcl-vm/src/exec.rs`.

## Legend

- `[x]` — executed by `tcl-vm` (`exec.rs` has a dispatch arm for it); a
  trailing `(note N)` marks a documented partial or deliberate divergence,
  listed under **Notes** below.
- `[~]` — present in the `tcl-bytecode` `Op` enum, so codegen can emit it, but
  **not executed** by the VM.
- `[ ]` — **not in** the `Op` enum; it has to be added to `tcl-bytecode` first,
  matching the C Tcl mnemonic and operands.

The rows follow Tcl 9.0.4's `tclInstructionTable[]` ordering and operand
widths. The `opcode_family_partition_total` test
(`rust/tcl-bytecode/src/lib.rs`) keeps every enum variant routed to exactly one
mnemonic family and size class, and `rust/tcl-vm/tests/opcode_c_parity.rs` +
`opcode_catch_parity.rs` pin the C-semantics contracts.

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
- [x] `startCommand` (note 5)
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
- [x] `tclooSelf` (note 4)
- [x] `tclooClass`
- [x] `tclooNamespace`
- [x] `tclooIsObject`
- [x] `arrayExistsStk` (note 6)
- [x] `arrayExistsImm` (note 6)
- [x] `arrayMakeStk`
- [x] `arrayMakeImm`
- [x] `invokeReplace`
- [x] `listConcat`
- [x] `expandDrop`
- [x] `foreach_start`
- [x] `foreach_step`
- [x] `foreach_end`
- [x] `lmap_collect` (note 7)
- [x] `strtrim`
- [x] `strtrimLeft`
- [x] `strtrimRight`
- [x] `concatStk`
- [x] `strcaseUpper` (note 8)
- [x] `strcaseLower` (note 8)
- [x] `strcaseTitle` (note 8)
- [x] `strreplace`
- [x] `originCmd`
- [x] `tclooNext` (note 4)
- [x] `tclooNextClass` (note 4)
- [x] `yieldToInvoke`
- [x] `numericType`
- [x] `tryCvtToBoolean`
- [x] `strclass`
- [x] `lappendList`
- [x] `lappendListArray`
- [x] `lappendListArrayStk`
- [x] `lappendListStk`
- [x] `clockRead` (note 9)
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
2. **`returnImm`/`syntax` carry C's `(code, level)` operands**, stack order and
   immediates both C-exact. `INST_RETURN_IMM` reads
   `OBJ_AT_TOS` as the options dict and `OBJ_UNDER_TOS` as the result
   (`CompileReturnInternal` pushes the options *last*), which is what our
   codegen emits and our arm pops; C's `returnStk` is the **opposite** order —
   result on top, options under — and ours matches that too.

   The arm implements `TclProcessReturn` (`tclResult.c:777-785`): `level
   != 0` raises `TCL_RETURN` carrying `-code`/`-level` for the proc-boundary
   countdown, and `level == 0` makes the completion *be* `code`, so `(0, 0)`
   falls through to the next instruction with the result on the stack rather
   than returning. It delegates to `cmd_return` with the operands appended
   after `-options`, so the immediates win over the literal dict exactly as
   C's `TclMergeReturnOptions` out-params do.

   The encodings codegen emits, per `TclCompileReturnCmd` /
   `CompileReturnInternal` / `TclCompileErrorCmd`: a plain `return` in a proc
   with no enclosing catch folds to `done`; a plain `return` anywhere else is
   `(0, 1)` (C's default `-level` is 1); `-code error` is `(1, 1)`; `-code
   return` merges to `-code ok -level L+1`, i.e. `(0, 2)`; `-code break` /
   `-code continue` / `-code N` are `(3, 1)` / `(4, 1)` / `(N, 1)`; and `error
   msg` is `(1, 0)`. `return -level 0 $x` stays `(0, 0)` where C elides the
   instruction outright — equivalent under the corrected semantics, since
   `(0, 0)` just leaves the result on the stack.

   Two peephole passes (`fold_tail_return_to_done`, `strip_unused_start_cmd`)
   are keyed on the plain-return pair `(0, 1)`; re-key them with the pair or
   they silently stop firing, which changes the emitted bytes.

   Still divergent: the runtime `return` **command** (`cmd_return`) omits
   `TclMergeReturnOptions`' `-code return` → `-code ok -level L+1`
   normalisation (`tclResult.c:969-974`). A plain proc-body `return -code …`
   compiles to a generic invoke rather than `returnImm`, so
   `proc p {} {return -code return}` escapes as `TCL_RETURN` instead of
   returning from `p`'s caller.
3. **`dictRecombineImm`/`dictRecombineStk` ignore the key path** — the
   compiled `dict with` writeback handles a top-level dict variable only, not
   a nested `dict with d k {…}` path (pre-existing in the `Imm` form; the
   `Stk` form mirrors it so the two cannot drift).
4. **TclOO context test** is "an OO frame is on the VM's call stack" (the
   `next`/`self` commands' existing rule), slightly looser than C's
   `FRAME_IS_METHOD` — a plain proc *called from* a method still counts.
   Opcode and command surfaces agree with each other.
5. **`startCommand`** is inert (its length/cmd-count operands are carried for
   disassembly parity; the VM needs no interp-epoch recheck).
6. **`arrayExistsImm`/`arrayExistsStk` skip C's `TclCheckArrayTraces`** — the
   VM records `array` trace ops but fires only read/write/unset traces
   anywhere, so the opcodes stay consistent with the VM's `array exists`.
7. **`lmap_collect`** keeps the accumulator in the VM's loop state
   (`ForeachState.accum`) where C uses a compiler temp local; observable
   behaviour matches.
8. **Case-mapping ops** use Unicode *simple* (per-char) mappings like C's
    `Tcl_UniCharToUpper` (`ß` stays `ß`), including C's byte-length guard and
    the Georgian Mtavruli titlecase exception; known residual: `İ` (U+0130)
    stays `İ` where C's table lowercases to `i` (Rust exposes no 1:1 simple
    mapping for it).
9. **`clockRead 0` (clicks) returns microseconds** — the same
    `host.clock().now_micros()` backend the VM's `clock clicks` uses, so
    opcode and command agree (C uses `TclpGetWideClicks`).

- The `Op` enum also carries opcodes that are **not** C Tcl instructions and
  are intentionally outside this checklist: the ten `irule*` dialect operators
  (emitted for iRules expressions) and five extras
  (`land`/`lor`/`lnot`/`strreverse`/`strrepeat`). All fifteen are executed.
  `land`/`lor` are never emitted by codegen
  (`&&`/`||` compile to short-circuit jump sequences instead), but the VM's
  dispatch `match` is exhaustive over the whole `Op` enum
  (`rust/tcl-vm/tests/opcode_dispatch_coverage.rs`), so they carry
  real eager-boolean dispatch arms rather than being dead.
- Variable opcodes come in `Scalar1/Scalar4/ScalarStk/Array1/Array4/ArrayStk/Stk`
  families — every family member C Tcl emits is covered.
- `foreach_start`/`foreach_step`/`foreach_end` carry the `ForeachInfo` aux
  (loop-var groups) out-of-band on the instruction (`foreach_vars`), and
  `dictUpdateStart`/`dictUpdateEnd` their `DictUpdateInfo` analogue
  (`dict_vars`), keeping operand bytes C-shaped.
