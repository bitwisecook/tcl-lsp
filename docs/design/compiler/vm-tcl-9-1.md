# The VM on a Tcl 9.1 base

**Status:** designed 26 November 2026, plan item 312 (owner decision, 26 November 2026: the VM moves to Tcl 9.1 and later).

**Builds on:**
- `tcl-bytecode`'s `Op` and the compiler's release-aware codegen (`tcl-codegen`);
- the release-fact tables of items 79 and 202;
- the pinned oracles in `rust/tcl-dialect/data/reference-toolchains.tsv` (9.1 is `9.1.0`, tag `core-9-1-0`).

**Feeds:**
- item 311, [serialised bytecode](bytecode-image.md), in the tbcx format (format 93), whose units carry 9.1-numbered code bytes. tbcx validates code against the running C Tcl 9.1 table, so slots 0–210 must equal 9.1.0's;
- item 88's engine convergence, through `runtime/rust`'s shared use of `tcl-bytecode`.

## Problem

The VM runs one opcode set for every target release, and older releases are reached through dialect gating. The docs call that set "Tcl 9.0.2" (`rust/tcl-bytecode/src/lib.rs:22`, `docs/design/runtime/tclvm-opcode-status.md`). The code has already moved past that.

**`Op` today** (`rust/tcl-bytecode/src/op.rs:27`) has 225 variants:

| Group | Variants |
|---|---|
| C Tcl 9.0.4 opcodes (`tclInstructionTable[]`, 191 entries) | 191 |
| Opcodes added in Tcl 9.1 | 17 of 20 |
| A second variant for slot 188 (`LREPLACE`, beside 9.0's `LREPLACE4`) | 1 |
| Private opcodes, in no C Tcl 9.x table: `CALL_FUNC1`, `LAND`, `LOR`, `LNOT`, `STR_REPEAT`, `STR_REVERSE` and ten `IRULE_*` | 16 |

Two parts are missing:
- **No numbering.** `Op` has mnemonics (`op.rs:419`), sizes (`op.rs:960`) and a 9.1 wide-form map (`op.rs:1092`), but no opcode byte. Nothing can emit or read C Tcl code bytes.
- **Three 9.1 opcodes are absent:** `dictRemove`, `arithSeries` and `foreach_index`.

**How 9.1 is reached today.** Codegen compiles in the 9.0 shape. For a 9.1 target, a post-pass rewrites each instruction to its wide form (`rust/tcl-codegen/src/codegen/operand_width.rs:43`, keyed on `TclVersion::emits_wide_instructions_only`, `rust/tcl-dialect/src/version/release_behaviour.rs:380`). The `bytecode-compare` goldens cannot show 9.1 at all: they are release-blind, so 9.1 is compared through cached `tcl dis --dialect tcl9.1` output (`.claude/skills/bytecode-compare/SKILL.md`). The corpus has 72 fixtures (`rust/tcl-compiler/tests/fixtures/codegen/matching/`). None of them exercises a command that 9.1 newly compiles.

## What changed from 9.0 to 9.1, from the sources

These come from `tmp/tcl9.0.4/generic/` and `tmp/tcl9.1.0/generic/`. The tables were compared slot by slot, with `tclCompile.c:44` (9.0.4) against `tclCompile.c:65` (9.1.0). The design is [TIP 720](https://core.tcl-lang.org/tips/doc/trunk/tip/720.md).

**Numbering.** Slots 0–190 keep their numbers. 9.1 appends slots 191–210 (211 in all). So 9.0 code bytes are valid 9.1 code bytes. 8.x numbering is different: 8.6.18 differs from 9.0 at 150 of its 190 slots, because 9.0 dropped `lor`/`land` at slots 40–41.

**Renamed** (same slot, same width, suffix dropped): `push4`→`push`, `invokeStk4`, `loadScalar4`, `loadArray4`, `storeScalar4`, `storeArray4`, `jump4`, `jumpTrue4`, `jumpFalse4`, `beginCatch4`, `appendScalar4`, `appendArray4`, `lappendScalar4`, `lappendArray4`, `lreplace4`.

**Deprecated** (`DEPRECATED_OPCODE`, `tclCompile.h:639`): every one-byte-operand form, `loadScalarStk`, `storeScalarStk`, `returnCodeBranch` (now `returnCodeBranch1`), and the one-byte `tailcall`, `tclooNext` and `tclooNextClass`.
- The 9.1 compiler no longer emits them.
- `tclExecute.c` still executes them unless the build defines `REMOVE_DEPRECATED_OPCODES` (`tclExecute.c:19`). Such a build "cannot load Tcl 9.0 bytecodes" (TIP 720).

**Reused names.** `tailcall`, `tclooNext` and `tclooNextClass` each name two slots, a deprecated one-byte one (143, 172, 173) and a new four-byte one (195–197). Anything that keys an opcode by mnemonic is wrong on 9.1. Width-explicit names, such as our `TCLOO_NEXT4`, stay unambiguous.

**Operand types retyped**, with the same width: `unset*` carry `OPERAND_UNSF1`, `lreplace` carries `OPERAND_LRPL1` and `clockRead` carries `OPERAND_CLK1`.

**New opcodes** (slots 191–210) and what compiles to them:

| Slot | Opcode | Emitted by (9.1 source) | In `Op` |
|---|---|---|---|
| 191–194 | `incrScalar`, `incrArray`, `incrScalarImm`, `incrArrayImm` (LVT4) | `incr` (`tclCompCmdsGR.c:483`) | yes |
| 195 | `tailcall` (UINT4) | `tailcall` (`tclCompCmdsSZ.c:2970`) | `TAILCALL4` |
| 196–197 | `tclooNext`, `tclooNextClass` (UINT4) | `next`, `nextto` (`tclCompCmdsGR.c:3267`, `:3349`) | `TCLOO_NEXT4`, `TCLOO_NEXT_CLASS4` |
| 198 | `swap` | `array set`, dict compilers | yes |
| 199 | `errorPrefixEq` | `try … trap` (`tclCompCmdsSZ.c:3488`) | yes |
| 200 | `tclooId` | `info object creationid` (`tclCompCmdsGR.c:736`) | yes |
| 201 | `dictPut` | `dict replace` and others | yes |
| 202 | `dictRemove` | `dict remove` (`tclCompCmds.c:1358`) | **no** |
| 203 | `isEmpty` | `string is` (`tclCompCmdsSZ.c:632`) | yes |
| 204 | `jumpTableNum` (new aux type `JumptableNumInfo`) | `subst`, `switch -integer`, `try` | yes |
| 205 | `tailcallList` | `tailcall` with `{*}` | yes |
| 206–207 | `tclooNextList`, `tclooNextClassList` | `next`/`nextto` with `{*}` | yes |
| 208 | `arithSeries` (flags UINT1) | `lseq` (`tclCompCmdsGR.c:1788`) | **no** |
| 209 | `uplevel` | `uplevel` (`tclCompCmdsSZ.c:4662`) | yes |
| 210 | `foreach_index` (UINT4 UINT4) | `lfilter` (`tclCompCmds.c:3228`) | **no** |

**Newly compiled commands in 9.1:** `dict remove`, `dict replace`, `info object creationid`, `ledit`, `lfilter`, `lpop`, `lseq` and `uplevel`, each with a `TclCompile*Cmd` that 9.0.4 lacks. `switch -integer` ([TIP 730](https://core.tcl-lang.org/tips/doc/trunk/tip/730.md)) compiles through `IssueSwitchNumJumpTable`.

**Semantics with no new opcode.** `lseq` loses its `expr` behaviour ([TIP 746](https://core.tcl-lang.org/tips/doc/trunk/tip/746.md)), which is visible through `arithSeries`. The full list is in `tmp/tcl9.1.0/changes.md`. `subst` options, `timer`, `unicode`, the C99 math functions and regexp `\z` are command-level, and they stay registry facts.

**`ByteCode` struct.** Its fields are unchanged from 9.0.4. Only `createTime` (under `TCL_COMPILE_STATS`) changes type. The command-location encoding (`EncodeCmdLocMap`) is unchanged apart from a `Tcl_Size` local. The aux types are `ForeachInfo`, `NewForeachInfo`, `DictUpdateInfo` and `JumptableInfo`, plus the new `JumptableNumInfo` (`tclCompCmds.c:92`, `tclCompCmdsSZ.c:109`). `ForeachInfo`'s temp fields become `Tcl_LVTIndex`, an alias of `Tcl_Size`, so their width is unchanged.

## Design

### The opcode set is C Tcl 9.1's, numbered

`Op` keeps its width-explicit variant names: `PUSH4` stays, not `PUSH`. 9.1 reuses mnemonics across widths, and the VM has 784 `Op::` uses. Renaming every variant buys nothing a table cannot give. What changes is that every variant gets a byte:

```rust
// rust/tcl-bytecode/src/opcode_table.rs — generated, do not edit
impl Op {
    pub const fn code(self) -> u8;                 // C Tcl 9.1 slot, or a private code
    pub const fn from_code(code: u8) -> Option<Op>;
    pub const fn operands(self) -> &'static [OperandKind]; // LIT4, LVT1, UNSF1, AUX4, ...
}
/// Which opcodes a target release may contain, in 9.1 numbering.
pub fn admitted(release: tcl_dialect::TclVersion, op: Op) -> bool;
```

- **C slots 0–210** are generated from `tmp/tcl9.1.0/generic/tclCompile.c`'s `tclInstructionTable[]`, by name plus width. `LREPLACE` and `LREPLACE4` both map to 188.
- **Private opcodes** take codes from 255 downward. The generator fails if C's table reaches the private floor, so a later C release cannot collide with them silently.
- **Admission per release** is a generated table (`rust/tcl-bytecode/data/opcodes-by-release.tsv`), in the style of items 79 and 202:
  - each pinned release's `tclInstructionTable[]`, mapped by name and width onto 9.1 numbering;
  - plus the private opcodes that the release's dialect admits (8.4: `CALL_FUNC1`, `LAND`, `LOR`, `LNOT`; iRules: `IRULE_*`).

  Codegen asserts admission, and item 311's loader rejects a unit that breaks it.
- **Export lowering** is a column of the same table. It gives, per private opcode, the standard 9.1 sequence that item 311's interop export substitutes (`LNOT`→`not`, `LAND`/`LOR`, `STR_REPEAT`/`STR_REVERSE`), or none (`CALL_FUNC1`, `IRULE_*`).

### Semantics base: 9.1 for every opcode

Every dispatch arm implements 9.1's `TclExecuteByteCode` behaviour. Where 9.0 and 9.1 differ for one opcode, 9.1 wins, and the older behaviour is a dialect fact the arm reads. No such opcode-level difference was found beyond the operand retyping above, so this is a rule for later releases, not a migration today.

- **Deprecated opcodes stay executable.** C Tcl 9.1 keeps them by default. Our 8.4–9.0 targets emit them, which is how their output matches their own `tclsh`.
- **`runtime/rust` uses `tcl-bytecode`'s tables** (`LiteralTable`, `LocalVarTable`, `NativeExpressionNumberLiteral`: `runtime/rust/src/interp/native_body_artifact.rs:89`) but not `Op`. The numbering change does not touch it. Its 9.1 command semantics are item 88's work.

### Older releases keep their behaviour through gating

Three gates, each with one owner:

1. **What compiles.** Whether a command compiles inline, and to what, for a given release is a registry fact on its `CommandSpec`'s lowering descriptor, with a release range. It is never a `match` in codegen (the registry rule in `AGENTS.md`). The eight newly compiled commands get 9.1 ranges. On ≤ 9.0 they stay invocations, so `lfilter` and `lseq` there remain whatever the registry says those releases do.
2. **Operand width.** Width selection replaces the widening post-pass. Codegen picks the form from `TclVersion` when it emits, so jump layout is computed once in the target's shape. Narrow forms for ≤ 9.0, wide forms for 9.1.
3. **Command semantics.** The `lseq` `expr` removal, `subst` options and the new math functions are gated in the builtins by the same dialect point as today.

### Codegen

- Add `DICT_REMOVE`, `ARITH_SERIES` and `FOREACH_INDEX` to `Op`, with VM arms.
- Add lowering for the eight newly compiled commands, plus `switch -integer`, reading the registry descriptors.
- Emit `ERROR_PREFIX_EQ` and `JUMP_TABLE_NUM` where 9.1 does. Our `try` and `subst` paths already partly do (`native_try.rs:202`, `native_substitution.rs:139`).
- Emit wide forms directly for 9.1, and retire `widen_operands`.

### Decisions

- **Width-explicit names, generated numbers.** This avoids churn and the 9.1 name reuse. The C table is the source of truth, and a `--check` gate keeps the generated file current.
- **One numbering for every target.** Serialised units (311) are 9.1-numbered even for 8.x targets. Units for targets below 9.1 are tbcx *extended* units, which C Tcl does not load. C Tcl byte identity against `getbytecode` is promised for 9.0 and 9.1 targets only, because 9.0's slots are a prefix of 9.1's. 8.x targets are checked by disassembly, as now. Only 9.1 interop units are C-loadable, through tbcx at an exact patchlevel.
- **Private codes from the top.** C grows upward, and the gap is checked.
- **Do not port `REMOVE_DEPRECATED_OPCODES`.** Older targets need those opcodes.

### Open questions

- **Is 312 still deferred?** The plan's scope puts VM work off `work`, but 311, which needs 312's numbering, lands before 306. *Settled by the owner.* The split proposed in the plan text below takes only the `tcl-bytecode` part (steps 1–3) onto `work`.
- **9.0 or 9.1 name in disassembly for ≤ 9.0 targets?** Today it is the release's own name (`format.rs:331`). Keep that. Nothing measured argues otherwise.

## Work list

1. **Opcode table generator.** Add `cargo xtask opcode-table`. It parses each pinned tree's `tclInstructionTable[]` and writes `opcode_table.rs` and `opcodes-by-release.tsv`, with `--check` in `xtask-check`.
2. **`Op` numbering.**
   - Add `code`/`from_code`/`operands`/`admitted`, plus the three missing variants.
   - Extend the `opcode_family_partition_total` test (`rust/tcl-bytecode/src/tests.rs:341`) to cover codes.
   - Fix the crate doc (`lib.rs:22`), which says "Tcl 9.0.2" and "only dependency is `tcl-syntax`". It has four.
3. **Byte encoder.** Add `encode`/`decode` between `FunctionAsm` and code bytes, after `layout::resolve_layout` (`layout.rs:108`). Item 311 consumes it.
4. **Registry descriptors.** Add release ranges for the eight newly compiled commands and `switch -integer`.
5. **Codegen.**
   - Emit-time width selection, then delete `operand_width.rs`'s post-pass.
   - Lowering for the new commands.
   - Admission asserts.
6. **VM arms** for `dictRemove`, `arithSeries` and `foreach_index`, with 9.1 semantics, and conformance rows in `rust/tcl-vm/tests/it_5/opcode_c_parity.rs`.
7. **Opcode status.** Rewrite `docs/design/runtime/tclvm-opcode-status.md` in 9.1 slot order (211 rows plus a private-opcode section).
8. **bytecode-compare.**
   - Add release-aware goldens for 9.1, so `-v 9.1` compares goldens and not a cache.
   - Add fixtures for each newly compiled command, for `switch -integer`, `try … trap`, `tailcall {*}`, `next {*}`, and `incr` on a proc with more than 256 locals.
   - Add a byte-level mode. It rebuilds C Tcl's bytes from `tcl::unsupported::getbytecode` instructions and their `pc` offsets (`tclDisassemble.c`, `tclBasic.c:415`), using pc deltas to tell apart the slots that share a mnemonic. It compares them with `encode`'s output.
9. **Pins.** `reference-toolchains.tsv` already pins 9.1.0. `check-tcl-reference-toolchains` gains the rule that the opcode table's source tree is the pinned 9.1 tree.
10. **`runtime/rust`.** Nothing until item 88. Then its `Frames`/`Namespaces` traits carry the 9.1 `uplevel` and `tailcall` semantics, and the differential suite gains the new fixtures.

## Verification

Done when:
- `Op` has a code for every C Tcl 9.1 slot, and `opcode-table --check` is green against the pinned 9.1.0 tree.
- Every one of the 211 slots has a VM arm. `tclvm-opcode-status.md` reads 211 executed.
- `bytecode-compare -v 9.1 all` matches on the extended corpus by disassembly, and its byte mode matches `getbytecode`'s bytes, or the fixture is on a listed divergence list.
- `-v 8.4` through `-v 9.0` show no new mismatches against the pre-change baseline (`failset.py`).
- The conformance runner's rows for the new commands pass against `tclsh` 9.1.0, and against 9.0.4 as non-compiled invocations, native and WASM.

Record:
- **Build cone.** Edits to `opcode_table.rs` rebuild `tcl-bytecode` and its dependants. Measure with item 259's method.
- **VM benchmarks.** The existing VM benchmark set on 9.1 targets before and after emit-time widening. TIP 720 expects larger bytecode, and we measure ours.

## Risks

- **Width selection moves jump layout.** Shortening runs before widening today. Emitting wide directly changes offsets in 9.1 output only. The goldens show it.
- **Mnemonic-keyed code breaks silently on 9.1.** Grep for opcode lookups by name (`format.rs`, the explorer, `tcl dis`) and route them through `code()`.
- **A 9.2 that compacts opcodes.** TIP 720 proposes a "level 2" that renumbers. That would be a new base. The generator makes it a regenerated table, not a hand port.
- **Private codes.** A unit holding a private code is a tbcx *extended* unit, which tbcx refuses at its format word. That is intended for `IRULE_*`. The admission table keeps them out of `tcl` dialect units.

## Corrections to the overview

- The first `bytecode-image.md` (the `tclbc` design) and `lib.rs:22` say `Op` "matches Tcl 9.0.2". It is the 9.0 set plus 17 of 9.1's 20 additions plus 16 private opcodes, and it has no opcode numbers. The reference pin for 9.0 is 9.0.4, not 9.0.2.
- `tcl-bytecode`'s crate doc lists `tcl-syntax` as its only dependency. `Cargo.toml` lists `tcl-dialect`, `tcl-lexer`, `tcl-runtime-api` and `tcl-syntax`.
- `tclvm-opcode-status.md` counts 191 instructions and cites `rust/tcl-bytecode/src/lib.rs` for `Op`. `Op` is in `op.rs`.
