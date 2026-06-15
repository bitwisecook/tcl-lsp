# TCLVM opcode status — C Tcl 9.0 instruction coverage

> **Goal:** the bytecode VM (`rust/tcl-vm`) executes the **same opcode set** as
> the C Tcl 9.0 bytecode engine (`tmp/tcl9.0.3/generic/tclExecute.c`,
> `tclCompile.c` `InstructionDesc`). This is the binary-compatibility checklist:
> all 191 C Tcl 9.0 instructions, ticked off as the VM implements them. Match
> C Tcl semantics exactly (operands, stack effect, error behaviour).

## Legend

- `[x]` — executed by `tcl-vm` (`exec.rs`).
- `[~]` — present in the `tcl-bytecode` `Op` enum (the codegen can emit it) but
  **not yet executed** by the VM.
- `[ ]` — **not yet in** the `Op` enum (needs adding to `tcl-bytecode` first,
  matching the C Tcl mnemonic/operands).
Status (auto-countable): **97 executed · 40 enum-only · 54 missing · 191 total**.
Keep this in sync when adding opcodes — update the row and the count.

## Instructions (C Tcl 9.0 `InstructionDesc` order)

- [x] `done`
- [x] `push1`
- [x] `push4`
- [x] `pop`
- [x] `dup`
- [x] `strcat`
- [x] `invokeStk1`
- [x] `invokeStk4`
- [~] `evalStk`
- [x] `exprStk`
- [x] `loadScalar1`
- [x] `loadScalar4`
- [ ] `loadScalarStk`
- [x] `loadArray1`
- [ ] `loadArray4`
- [x] `loadArrayStk`
- [x] `loadStk`
- [x] `storeScalar1`
- [x] `storeScalar4`
- [ ] `storeScalarStk`
- [x] `storeArray1`
- [ ] `storeArray4`
- [x] `storeArrayStk`
- [x] `storeStk`
- [x] `incrScalar1`
- [ ] `incrScalarStk`
- [ ] `incrArray1`
- [ ] `incrArrayStk`
- [x] `incrStk`
- [x] `incrScalar1Imm`
- [ ] `incrScalarStkImm`
- [ ] `incrArray1Imm`
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
- [~] `beginCatch4`
- [~] `endCatch`
- [~] `pushResult`
- [~] `pushReturnCode`
- [x] `streq`
- [x] `strneq`
- [x] `strcmp`
- [x] `strlen`
- [x] `strindex`
- [x] `strmatch`
- [x] `list`
- [x] `listIndex`
- [x] `listLength`
- [~] `appendScalar1`
- [ ] `appendScalar4`
- [ ] `appendArray1`
- [ ] `appendArray4`
- [ ] `appendArrayStk`
- [~] `appendStk`
- [~] `lappendScalar1`
- [ ] `lappendScalar4`
- [ ] `lappendArray1`
- [ ] `lappendArray4`
- [ ] `lappendArrayStk`
- [~] `lappendStk`
- [~] `lindexMulti`
- [x] `over`
- [~] `lsetList`
- [~] `lsetFlat`
- [x] `returnImm`
- [x] `expon`
- [~] `expandStart`
- [~] `expandStkTop`
- [~] `invokeExpanded`
- [x] `listIndexImm`
- [x] `listRangeImm`
- [x] `startCommand`
- [x] `listIn`
- [x] `listNotIn`
- [~] `pushReturnOpts`
- [x] `returnStk`
- [~] `dictGet`
- [~] `dictSet`
- [~] `dictUnset`
- [~] `dictIncrImm`
- [~] `dictAppend`
- [~] `dictLappend`
- [ ] `dictFirst`
- [ ] `dictNext`
- [ ] `dictUpdateStart`
- [ ] `dictUpdateEnd`
- [x] `jumpTable`
- [~] `upvar`
- [~] `nsupvar`
- [ ] `variable`
- [~] `syntax`
- [~] `reverse`
- [~] `regexp`
- [x] `existScalar`
- [ ] `existArray`
- [ ] `existArrayStk`
- [x] `existStk`
- [x] `nop`
- [ ] `returnCodeBranch`
- [ ] `unsetScalar`
- [ ] `unsetArray`
- [ ] `unsetArrayStk`
- [x] `unsetStk`
- [ ] `dictExpand`
- [ ] `dictRecombineStk`
- [ ] `dictRecombineImm`
- [~] `dictExists`
- [x] `verifyDict`
- [~] `strmap`
- [x] `strfind`
- [x] `strrfind`
- [x] `strrangeImm`
- [x] `strrange`
- [ ] `yield`
- [ ] `coroName`
- [~] `tailcall`
- [ ] `currentNamespace`
- [ ] `infoLevelNumber`
- [ ] `infoLevelArgs`
- [ ] `resolveCmd`
- [ ] `tclooSelf`
- [ ] `tclooClass`
- [ ] `tclooNamespace`
- [ ] `tclooIsObject`
- [ ] `arrayExistsStk`
- [x] `arrayExistsImm`
- [ ] `arrayMakeStk`
- [ ] `arrayMakeImm`
- [~] `invokeReplace`
- [x] `listConcat`
- [ ] `expandDrop`
- [x] `foreach_start`
- [x] `foreach_step`
- [x] `foreach_end`
- [ ] `lmap_collect`
- [x] `strtrim`
- [x] `strtrimLeft`
- [x] `strtrimRight`
- [~] `concatStk`
- [x] `strcaseUpper`
- [x] `strcaseLower`
- [x] `strcaseTitle`
- [~] `strreplace`
- [ ] `originCmd`
- [ ] `tclooNext`
- [ ] `tclooNextClass`
- [ ] `yieldToInvoke`
- [~] `numericType`
- [x] `tryCvtToBoolean`
- [~] `strclass`
- [~] `lappendList`
- [~] `lappendListArray`
- [~] `lappendListArrayStk`
- [~] `lappendListStk`
- [ ] `clockRead`
- [ ] `dictGetDef`
- [x] `strlt`
- [x] `strgt`
- [x] `strle`
- [x] `strge`
- [~] `lreplace4`
- [ ] `constImm`
- [ ] `constStk`
## Notes

- The `Op` enum also carries dialect-only opcodes (`irule*`) that are **not**
  C Tcl instructions; they are intentionally outside this checklist.
- `foreach_start`/`foreach_step`/`foreach_end`/`lmap_collect` need the
  `ForeachInfo` aux (loop-var groups) carried on the instruction + the implicit
  start→step / step→body jumps (`tclExecute.c` INST_FOREACH_*); see the VM
  milestone notes.
- Variable opcodes come in `Scalar1/Scalar4/ScalarStk/Array1/Array4/ArrayStk/Stk`
  families — the VM should cover each family member C Tcl emits.
