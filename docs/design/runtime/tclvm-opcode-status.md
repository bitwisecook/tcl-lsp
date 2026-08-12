# VM opcode coverage — the C Tcl 9.0 instruction set

Binary compatibility means the bytecode VM (`rust/tcl-vm`) executes the **same
opcode set** as the C Tcl 9.0 bytecode engine
(`tmp/tcl9.0.3/generic/tclExecute.c`, `tclCompile.c`'s `InstructionDesc`), with
matching operands, stack effect, and error behaviour. This is the inventory of
where each of the 191 instructions currently stands.

Coverage: **145 executed · 7 enum-only · 39 absent · 191 total**. It is
maintained by hand — adding an opcode means updating its row and the count in
the same change. The rows are checked against the `Op` enum in
`rust/tcl-bytecode/src/lib.rs` and the dispatch arms in
`rust/tcl-vm/src/exec.rs`.

## Legend

- `[x]` — executed by `tcl-vm` (`exec.rs` has a dispatch arm for it).
- `[~]` — present in the `tcl-bytecode` `Op` enum, so codegen can emit it, but
  **not executed** by the VM.
- `[ ]` — **not in** the `Op` enum; it has to be added to `tcl-bytecode` first,
  matching the C Tcl mnemonic and operands.

Two `[x]` rows deserve their caveat up front: `beginCatch4` and `endCatch`
share the no-op arm with `nop` and `startCommand`. They dispatch without
error, but the VM does not drive `catch` through an exception-range stack the
way C's engine does, so an emitter cannot rely on them to establish one.

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
- [x] `beginCatch4`
- [x] `endCatch`
- [x] `pushResult`
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
- [x] `appendScalar1`
- [x] `appendScalar4`
- [x] `appendArray1`
- [x] `appendArray4`
- [ ] `appendArrayStk`
- [~] `appendStk`
- [x] `lappendScalar1`
- [x] `lappendScalar4`
- [x] `lappendArray1`
- [x] `lappendArray4`
- [ ] `lappendArrayStk`
- [~] `lappendStk`
- [x] `lindexMulti`
- [x] `over`
- [x] `lsetList`
- [x] `lsetFlat`
- [x] `returnImm`
- [x] `expon`
- [x] `expandStart`
- [x] `expandStkTop`
- [x] `invokeExpanded`
- [x] `listIndexImm`
- [x] `listRangeImm`
- [x] `startCommand`
- [x] `listIn`
- [x] `listNotIn`
- [x] `pushReturnOpts`
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
- [ ] `variable`
- [x] `syntax`
- [x] `reverse`
- [x] `regexp`
- [x] `existScalar`
- [ ] `existArray`
- [ ] `existArrayStk`
- [x] `existStk`
- [x] `nop`
- [ ] `returnCodeBranch`
- [x] `unsetScalar`
- [x] `unsetArray`
- [ ] `unsetArrayStk`
- [x] `unsetStk`
- [x] `dictExpand`
- [ ] `dictRecombineStk`
- [x] `dictRecombineImm`
- [x] `dictExists`
- [x] `verifyDict`
- [~] `strmap`
- [x] `strfind`
- [x] `strrfind`
- [x] `strrangeImm`
- [x] `strrange`
- [ ] `yield`
- [ ] `coroName`
- [x] `tailcall`
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
- [x] `invokeReplace`
- [x] `listConcat`
- [ ] `expandDrop`
- [x] `foreach_start`
- [x] `foreach_step`
- [x] `foreach_end`
- [x] `lmap_collect`
- [x] `strtrim`
- [x] `strtrimLeft`
- [x] `strtrimRight`
- [x] `concatStk`
- [x] `strcaseUpper`
- [x] `strcaseLower`
- [x] `strcaseTitle`
- [x] `strreplace`
- [ ] `originCmd`
- [ ] `tclooNext`
- [ ] `tclooNextClass`
- [ ] `yieldToInvoke`
- [x] `numericType`
- [x] `tryCvtToBoolean`
- [x] `strclass`
- [x] `lappendList`
- [~] `lappendListArray`
- [~] `lappendListArrayStk`
- [~] `lappendListStk`
- [ ] `clockRead`
- [ ] `dictGetDef`
- [x] `strlt`
- [x] `strgt`
- [x] `strle`
- [x] `strge`
- [x] `lreplace4`
- [ ] `constImm`
- [ ] `constStk`
## Notes

- The `Op` enum also carries opcodes that are **not** C Tcl instructions and are
  intentionally outside this inventory: the nine dialect-only `IRULE_*`
  opcodes, plus `LAND`, `LOR`, `STR_REPEAT`, and `STR_REVERSE` (C Tcl builds
  those from jumps or command calls rather than dedicated instructions).
- `not` maps onto the enum's `LNOT`. The enum also has a separate `NOT`
  variant, which no row here claims.
- Variable opcodes come in `Scalar1` / `Scalar4` / `ScalarStk` / `Array1` /
  `Array4` / `ArrayStk` / `Stk` families; every family member C Tcl emits needs
  covering, not just one representative. The remaining absences are
  concentrated in the `…Stk` members (`loadScalarStk`, `storeScalarStk`,
  `incrScalarStk`, `existArrayStk`, `unsetArrayStk`, …) and in the
  coroutine / TclOO / introspection instructions (`yield`, `coroName`,
  `yieldToInvoke`, `tcloo*`, `currentNamespace`, `infoLevel*`, `resolveCmd`,
  `originCmd`), which the VM implements at the command level rather than as
  bytecode.
- The seven enum-only opcodes are cases where the emitter has a name to target
  but the VM has no arm: `pushReturnCode`, `strmap`, `appendStk`, `lappendStk`,
  `lappendListStk`, `lappendListArray`, and `lappendListArrayStk`. Six of them
  are emitted by nothing. `pushReturnCode` is the exception — the compiler's
  control-flow codegen emits it — so a body that reaches it raises the VM's
  catch-all, `opcode pushReturnCode not implemented in tcl-vm`, at run time
  rather than at codegen time.

## Cross-references

- [`rust-vm-tier-parity.md`](rust-vm-tier-parity.md) — the generated per-stem
  tcltest scoreboard.
- [`tcl-test-tiers.md`](tcl-test-tiers.md) — the capability ladder the
  scoreboard groups by.
- [`../contracts/vm-bytecode-test-boundary.md`](../contracts/vm-bytecode-test-boundary.md)
  — identity versus behaviour at this boundary.
