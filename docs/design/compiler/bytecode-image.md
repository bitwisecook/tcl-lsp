# Serialised bytecode: tbcload's `.tbc` format on a Tcl 9.1 base

**Status:** designed 26 November 2026, plan item 311. It was redesigned around `tbcload` by owner decision on 26 November 2026, replacing the custom `tclbc` container.

**Builds on:**
- item 312, the [VM on a Tcl 9.1 base](vm-tcl-9-1.md), whose numbered opcode table and byte encoder this format carries;
- the [SpecTcl image](../registry/model/306-expect-spike.md), which stores Tcl hook bodies as source until this lands.

**Feeds:**
- precompiled hook bodies in pack images (items 305–307);
- AOT artefacts for the VM;
- cached compiled procedures for `tcl-db`.

## Problem

`tcl-bytecode` has no serialiser.
- A `ModuleAsm` (`rust/tcl-bytecode/src/lib.rs:520`) exists only in memory. Its instructions refer to symbolic labels (`FunctionAsm::labels`, `lib.rs:194`), and `Op` has no opcode bytes (see 312).
- Every process that runs a hook body, an AOT module or a cached procedure compiles it again.
- The VM can't load a module compiled elsewhere.

The owner chose `tbcload`'s format as the serialised form, and Tcl 9.1 as the VM base. The research below shows that these two decisions meet a gap: no maintained `tbcload` supports Tcl 9.1. The design keeps `tbcload`'s format and owns a versioned 9.1 extension of it.

## Research: what `tbcload` is

Sources:
- `tbcload` 2.0a0 (`cmpRead.c`, `cmpInt.h`);
- `tclcompiler` 2.0 (`cmpWrite.c`);
- both read from the `wip` branch of the Codeberg forks, last commit 23 November 2026.

### The file

A `.tbc` file is a Tcl script:

```
if {[catch {package require tbcload <version>} err] == 1} {
    return -code error "[info script]: <message> -- $err"
}
tbcload::bceval {
TclPro ByteCode 3 2.0a0 9.0
<one ByteCode record>
}
```

**The signature line** is `TclPro ByteCode <format> <writer x.y.z> <Tcl major.minor>`.
- The format was 1 for Tcl 8.0–8.3 and 2 for 8.4 and later (comment in `TbcloadInit`). It is 3 in 2.0.
- The reader rejects any format number but its own ("unsupported bytecode version: N").
- It rejects a file whose Tcl major differs from the running one, or whose minor is higher.

**Integers** are decimal tokens read as `Tcl_Size` (`ExtractTclSize`), so they are 64-bit on 9.x.

**Byte sequences** are a decimal length line, then a modified ASCII85:
- 4 bytes become 5 characters, least significant digit first, so a short last group drops its trailing `!`s;
- `z` stands for a zero word;
- the alphabet replaces `"`, `$`, `[`, `\` and `]` with `v`, `w`, `x`, `y` and `|`, so the payload is a safe brace-quoted Tcl word;
- lines are 72 characters.

### The ByteCode record

The record starts with 13 integers: `numCommands`, `numSrcBytes`, `numCodeBytes`, `numLitObjects`, `numExceptRanges`, `numAuxDataItems`, `numCmdLocBytes`, `maxExceptDepth`, `maxStackDepth`, then the four command-map array sizes. A source-map size of `-1` means no source is shipped.

The parts follow in this order:

| Part | Encoding |
|---|---|
| code, codeDelta, codeLength (srcDelta, srcLength if present) | byte sequences |
| literals | one tag each, below |
| exception ranges | count, then `L` or `C`, `nestingLevel codeOffset numCodeBytes breakOffset continueOffset catchOffset` |
| aux data | count, then `J` JumptableInfo, `D` DictUpdateInfo or `f` NewForeachInfo |

**Literal tags:**

| Tag | Meaning |
|---|---|
| `i` | int, as text |
| `d` | double, as text |
| `s` | length plus raw string |
| `x` | ASCII85 string |
| `b` | boolean (the reader accepts it; the writer never emits it) |
| `c` | a nested ByteCode record |
| `p` | a procedure body: a ByteCode record, `numArgs`, `numCompiledLocals`, then per local: name, `frameIndex`, has-default, a flag mask (bit 8 `VAR_ARGUMENT`, bit 9 `VAR_TEMPORARY`) and the default literal |

The writer emits `i` and `d` only for objects already of those types, and `x` for everything else: lists, dicts, bignums and booleans keep only their string form.

**Procedures and nested bodies.**
- While the top-level script compiles, `tclcompiler` hooks `proc`'s compile procedure. For each literal `proc name args body` at the top level, it replaces the body literal with a `p` record and rewrites the `proc` literal to `tbcload::bcproc`.
- It restores the hook before compiling the bodies (`cmpWrite.c`, the "restore the original compile proc" step).
- So a procedure inside another procedure or a `namespace eval` body stays as source text, and so do lambdas and TclOO method bodies. Compiled bodies show a sentinel as their string form ("`# Compiled -- no source code available`…").

**No opcode check.** The reader copies code bytes verbatim, with no opcode or jump validation.

### Where it is maintained, and which releases it supports

| Line | Where | Tcl releases | State |
|---|---|---|---|
| TclPro 1.x (Ajuba, 1998–2000) | SourceForge, [tcl-lang.org/software/tclpro](https://tcl-lang.org/software/tclpro/compiler.html) | 8.0–8.4 | Dead. The open-source copy broke on 8.5's `{*}` ([wiki](https://wiki.tcl-lang.org/revision/tbcload?V=19)) |
| ActiveState 1.6–1.7 (Tcl Dev Kit) | Closed source in TDK/ActiveTcl; the code was later released as BSD-3 (copyright 2018 in the sources) | 8.4–8.6 | Superseded by the fork below |
| `tbcload`/`tclcompiler` 2.0a0 | [github.com/tcltk-depot/tbcload](https://github.com/tcltk-depot/tbcload), [codeberg.org/tcltk/fork-tbcload](https://codeberg.org/tcltk/fork-tbcload), [fork-tclcompiler](https://codeberg.org/tcltk/fork-tclcompiler) | 8.6 and 9.0. Files do not cross majors | Alpha. README: "**This package does not support Tcl 9.1 and later.** Use the tbcx extension instead." |
| KitCreator | [rkeene/KitCreator](https://github.com/rkeene/KitCreator) | — | It ships no `tbcload` package |
| core.tcl-lang.org | No `tbcload` repository there (404 at the time of reading) | — | — |
| `tbcx` 1.2.1 (August 2026) | [bagnongithub/tbcx](https://bagnongithub.github.io/tbcx), MIT | 9.1 only, exact patchlevel match | A different, binary format (magic `TBCX`, format 93). It covers procs, TclOO methods, lambdas and nested bodies, and its literal tags include bignum, bytearray, dict, list and wide int |

### What `tbcload` cannot represent on 9.1

- **`JumptableNumInfo`.** This is 9.1's new aux type (`tclCompCmdsSZ.c:109`), used by `subst`, `switch -integer` and `try`. The writer panics on an unknown aux type (`EmitAuxDataArray`). [TIP 720](https://core.tcl-lang.org/tips/doc/trunk/tip/720.md) warns that save/load code needs changes for it.
- **Format 3 is fixed.** A 9.1 file needs a new number, and a 2.0a0 reader then refuses it.
- **Byte-array literals.** A non-UTF-8 byte literal goes through `Tcl_NewStringObj`. `tbcx` added a tag for this reason.
- **Nested procedures, lambdas, TclOO methods and `namespace eval` bodies.** These stay as source.
- **What does *not* block 9.1:**
  - 64-bit sizes, which are decimal `Tcl_Size`;
  - the `ByteCode` struct, whose serialised fields are unchanged from 9.0.4 to 9.1.0 (312);
  - the new opcodes, which the reader passes through unchecked.

## Design

### The payload: `.tbc`, format 9101

Our payload is `tbcload`'s record grammar, with a versioned extension.

**The format number is `91NN`:** base Tcl 9.1, extension revision `NN`. The first revision is `9101`.
- The signature line keeps `tbcload`'s shape, so a C Tcl `tbcload` parses it and then refuses it cleanly as an unsupported version.
- A later base (for example a compacted 9.2) is `92NN`.
- Our reader accepts only the revisions it implements. An unknown number is `ImageError::UnknownFormat`, and the caller compiles from source.

```
TclPro ByteCode 9101 1.0.0 <target release major.minor, 8.4 … 9.1>
```

The format-3 grammar is unchanged, plus these extensions in revision 01:

| # | Extension | Why |
|---|---|---|
| E1 | Code bytes use 312's numbering: C Tcl 9.1 slots 0–210, private opcodes from 255 down | One numbering for every target |
| E2 | Aux tag `N`, JumptableNumInfo: count, then `key offset` pairs. `J` and `N` entries are sorted by key | 9.1's numeric jump table; deterministic output |
| E3 | Literal tag `a`: a byte array, as an ASCII85 byte sequence | Non-UTF-8 literals survive |
| E4 | `c` for any nested body our codegen precompiles. Literal tag `l`, a lambda: args, namespace, then a `p` record | Bodies `tclcompiler` leaves as source |
| E5 | After the top-level `c` record: `B`, the top level as a procedure body (a `p` record), then `P count` and per procedure its name, namespace, parameters (each `x`) and a `p` record, ordered by `ProcedureId` | Mirrors `ModuleAsm` (`top_level`, `top_level_body`, `procedures`). The `proc` literal is not rewritten to `bcproc` |
| E6 | The source map is present (sizes ≥ 0) when the unit ships its source | `errorInfo` and the LSP need real positions |

Nothing of ours goes into the payload beyond E1–E6.

### Two codecs over one record model

`tcl-bytecode` gains a `tbc` module. It needs no compiler and no VM, so the SpecTcl loader, `tcl-db` and both engines can use it.

```rust
pub mod tbc {
    pub struct Payload { pub target: TclVersion, pub top: ByteCodeRec,
                         pub top_body: ProcBodyRec, pub procedures: Vec<ProcRec> }
    pub fn from_module(m: &ModuleAsm) -> (Payload, UnitMeta);
    pub fn into_module(p: Payload, meta: UnitMeta) -> Result<ModuleAsm, ImageError>;
    pub fn write_tbc(p: &Payload) -> String;            // ASCII85, with the bceval preamble
    pub fn read_tbc(text: &str) -> Result<Payload, ImageError>;
    pub fn write_raw(p: &Payload) -> Vec<u8>;           // same records, unencoded
    pub fn read_raw(bytes: &[u8]) -> Result<Payload, ImageError>;
    pub fn validate(p: &Payload, meta: Option<&UnitMeta>) -> Result<(), ImageError>;
}
```

**The raw codec** is the same records in the same order:
- magic `TBCR`, then format `u32` (`9101`) and the target release;
- the same one-letter tags as `u8`;
- integers as LEB128, zigzag for fields that may be `-1`;
- byte sequences as a length plus raw bytes.

Raw and ASCII are transcodings of one `Payload`, and a test holds `read_raw(write_raw(p)) == read_tbc(write_tbc(p)) == p`.

### Our metadata: a wrapper record, not the payload

In pack images each unit is a wrapper record:

```
TbcUnit  { meta_format: u16, meta: UnitMeta, payload: raw tbc bytes }
UnitMeta { profile_id, dialect_point, source_namespace, source_hash,
           plain_command_dispatch, specialisation stamps (PackFactStamp list,
           ArtefactIdentityManifest), functions: Vec<FunctionMeta> }
FunctionMeta { native compilation (preflight, failure, prerequisites, required
           local layout), literal actions and allocation, command and procedure
           bindings, site claims, error regions, splice ranges, body base line,
           provenance (name, namespace context, parameters, body source) }
```

- `functions` is indexed by payload order: 0 is `top`, 1 is `top_body`, and `2..` are `procedures`.
- `meta_format` is versioned on its own, so the payload format moves only with C Tcl.

**A standalone `.tbc`** carries its `UnitMeta` in a sidecar, `<name>.tbc.meta`, in the raw encoding. Without one, the file loads as *foreign*:
- its profile is the plain `tcl` profile of the header's release;
- it never gets native compilation and has no provenance;
- it is always validated.

### Decisions

1. **The serialised form is `tbcload`'s, extended and versioned as `91NN`.** This is the owner's decision. The extension is ours to maintain, because upstream stops at 9.0.
2. **No C Tcl interop in 311, in either direction.**
   - Interop applies only where a maintained `tbcload` supports the release, and none supports 9.1, our base.
   - 9.0 is technically reachable, because its slots are a prefix of 9.1's (312). But it would mean a second, format-3 writer for an alpha loader, for a release below our base.
   - Reading C Tcl's format-3 files is left to a later item, if there is demand.
   - Code-byte identity with `tclsh` 9.1 is still required, because it is codegen correctness. It also keeps an exporter possible (open question 1).
3. **Pack images store the raw codec, and only `.tbc` files use ASCII85.**
   - ASCII85 alone is 5/4 of the bytes, plus a newline per 72 characters: about 1.27 times. Integers become decimal text.
   - Decoding costs a table lookup and base-85 arithmetic per character.
   - The images are internal, so they gain nothing from the text form.
   - The measurement under Verification can reverse this.
4. **Metadata in the wrapper, never the payload.** The payload stays a `tbcload`-grammar stream that our dumper and a future exporter read without knowing our fields. It is a wrapper rather than a sidecar in pack images, because a unit and its metadata must be replaced together.
5. **Deterministic bytes.** Sorted jump tables and procedure order mean the same module writes the same bytes, which `cargo xtask spec-images --check` needs (306).
6. **Validation on load, before anything executes.** A unit that fails any of these checks never runs:
   - the format number is known;
   - the target release is admitted for the profile;
   - every count and byte-sequence length matches;
   - every opcode is known and admitted for the target release (312's table);
   - jump targets fall on instruction boundaries inside the code;
   - literal, LVT and aux indices are in range;
   - each aux kind matches its opcode (`jumpTable`→`J`, `jumpTableNum`→`N`, `foreach_start`→`f`, `dictUpdate*`→`D`);
   - exception ranges lie within the code and nest no deeper than `maxExceptDepth`;
   - the recomputed stack depth is ≤ `maxStackDepth`;
   - the command map decodes to `numCommands` entries;
   - `UnitMeta` has one entry per payload function, and its regions, splices and claims are in range;
   - the profile id is known and matches the dialect point.

### Open questions

1. **Export to `tbcx` for C Tcl 9.1 interop?** `tbcx` is the maintained 9.1 route that the `tbcload` README points to. It is binary, MIT-licensed, and pinned to an exact patchlevel. *Settled by the owner.* The evidence for a yes is a user who needs our output in C Tcl 9.1.
2. **Must 311 land before 306?** 306 found one dynamic Tcl hook in the bundled packs and measured compile-at-first-call as negligible (`306-expect-spike.md`, "Dynamic Tcl hooks"). *Settled by* the load-against-compile measurement below. If load does not beat compile by a margin that matters at LSP start, 306 can proceed with source and 311 can follow.
3. **Do pack images ship body source?** It is needed for `info body` and for positions. *Settled by* the image-size measurement, with and without it.

## Work list

1. **312, steps 1–3:** the numbered opcode table, `admitted`, and `encode`/`decode`. Everything below needs them.
2. **Record model and ASCII codec** (`tbc::Payload`, `read_tbc`, `write_tbc`). As a format check, our reader parses a few `.tbc` files that `tclcompiler` 2.0a0 writes under `tclsh` 9.0.4, accepting format 3 in a test-only mode, without executing them. Those files are minimised execution evidence under `evidence/tcl/`.
3. **Raw codec**, and the transcoding test.
4. **`from_module`/`into_module` and `UnitMeta`**, covering every `FunctionAsm` field (`lib.rs:165`) and `ModuleAsm` field.
5. **`validate`**, with one rejection fixture per rule.
6. **Consumers:**
   - pack images store hook bodies as `TbcUnit` (305, 306);
   - the VM loads AOT modules;
   - `tcl dis` dumps `.tbc` and `TbcUnit`;
   - `tcl-db` caches procedures by source hash, profile and format, if measurement shows compile time matters.
7. **Fuzzing:**
   - a `tbc-decode` arm in `tcl-fuzz`: mutated payloads must be rejected or loaded, never panic or execute unvalidated;
   - its deterministic corrupted fixtures run in CI, and the campaign stays manual (`AGENTS.md`).
8. **Measurements**, as below.

## Verification

Done when:
- **Round trip.** `into_module(from_module(m)) == m`, and both codecs round-trip, for every module the bytecode-compare corpus produces, compiled for each target release 8.4–9.1.
- **Byte identity.** Code bytes for 9.1 targets match `tclsh` 9.1.0's (312's byte mode), and for 9.0 targets `tclsh` 9.0.4's, or the fixture is on a listed divergence list.
- **Format conformance.** The `tclcompiler` 2.0a0 fixtures parse.
- **Execution parity.** Running each script from its unit gives the same result as compiling it, in the VM native and in `tcl-vm-wasm`, as a differential test in the conformance runner.
- **Rejection.** Truncated input, a bad jump, an out-of-range index, an unknown format, an unadmitted opcode and a metadata mismatch are each rejected, and the fuzz arm runs clean for a recorded campaign.
- **Use.** Pack images store hook bodies as `TbcUnit`.

Measurements to record:
- **Codec decision.** Bytes and decode time per codec, native and WASM, over:
  - the corpus compiled for 9.1;
  - the bundled packs' hook section;
  - tcllib 2.0 compiled for 9.1, the large corpus, since 306 found one hook.

  Keep the raw codec unless it saves under 20% of bytes *and* decodes the whole bundled hook section less than 0.1 ms faster. If both hold, drop it and keep one codec.
- **Load against compile.** The time to load a hook body against compiling it (open question 2), and the hook section's size with and without source (open question 3).
- **Build cone.** `tcl-bytecode` edits, measured with item 259's method.

## Risks

- **The format is ours alone from 9.1 on.** Upstream stops at 9.0, and `tbcx` uses a different format. The `91NN` number and the reject-unknown rule keep that honest. The cost is that every C Tcl release with new aux data or literals needs an extension revision.
- **Upstream 2.0 is alpha.** Our format-3 conformance fixtures pin what we read, not what upstream will do.
- **The wrapper and payload drift.** Function-count and range checks in `validate` catch a mismatch on load.
- **ASCII85 cost on big `.tbc` files.** They are opt-in exports, so pack images never pay it.

## Corrections to the overview

- Plan item 311's text describes the superseded `tclbc` container, a `tbcload` research step and "byte identity with `tclsh` 9.0". The proposed replacement text is in this design's handoff to the coordinator.
- The earlier version of this doc says `Op` "matches Tcl 9.0.2". See 312's corrections.
