# Serialised bytecode: a binary image of compiled Tcl

**Status:** designed 26 November 2026, plan item 311.

**Builds on:**
- `tcl-bytecode`, whose opcode set (`Op`) matches Tcl 9.0.2;
- the [SpecTcl image](../registry/model/306-expect-spike.md), which stores Tcl hook bodies as source until this lands.

**Feeds:**
- precompiled hook bodies in pack images (items 305–307);
- AOT artefacts for the VM;
- cached compiled procedures for `tcl-db`.

## Problem

`tcl-bytecode` has no serialiser, and nothing versions its instruction set.

- A compiled `ModuleAsm` exists only in memory. Every process that runs a Tcl hook body, an AOT module or a cached procedure compiles it again.
- The SpecTcl 3 image therefore stores hook bodies as source and compiles them on first call (306's blocker).
- The VM can't load a module compiled elsewhere: for example, one compiled at build time into an editor bundle, or compiled for the WASM build and checked against native.

## Design: C Tcl's bytecode, in a versioned container

The instruction stream is C Tcl's own encoding. The `bytecode-compare` skill already checks our emitter's output against `tclsh` disassembly for 8.4–9.1, and `Op` matches Tcl 9.0.2. So the code bytes in the image are the bytes C Tcl would execute for the same source under that opcode set.

The tables around the code follow the shape of C Tcl's `ByteCode` structure:
- the literal array;
- compiled locals;
- exception ranges;
- auxiliary data (`foreach` info, jump tables, dict-update info);
- the command location map.

A reader who knows `tclCompile.h` can read the image.

C Tcl has no on-disk format of its own. TclPro's `tbcload` is the established serialisation of C Tcl bytecode. Its container and its support for Tcl 9 need checking before we promise interop. Step 4 is that check, and it decides the optional exporter.

### The container, `tclbc`

```
header      magic "TCLBC\0", container version u16, opcode set id (e.g. tcl-9.0.2),
            dialect point (family, release, build), endianness flag (little), section table
strings     one string table; every name and literal text is an offset into it
literals    typed: string | int | wide | bignum | double | boolean | list | dict | bytecode-ref,
            each with a canonical byte encoding (no host pointers, no float formatting)
functions   per function: name, code bytes (C Tcl encoding), max stack depth,
            compiled locals (name, flags, default), exception ranges, aux data, command map
metadata    our extras, which C Tcl's ByteCode has no field for: the dialect profile id,
            source namespace, native-compilation preflight and obligations,
            specialisation stamps, source hash
```

### Decisions

- **Offsets, not pointers.** Integers have fixed widths and are little-endian. There is one string table. Native and WASM builds load the same bytes.
- **Opcode-set id, not a guess.**
  - A VM that does not implement the image's opcode set refuses the image, and the caller compiles from source instead.
  - When a later C Tcl release changes an opcode, that is a new set id. It is never a reinterpretation of old bytes.
- **Literals are encoded canonically.** Doubles are stored as IEEE bits, never as formatted text. Bignums are stored as sign plus magnitude bytes. Lists and dicts are stored as their canonical string representation plus a typed hint.
  - This keeps shimmering behaviour identical to compiling at run time: the string representation is authoritative.
- **Validation on load.** Every jump target must fall on an instruction boundary. Every literal, local and aux index must be in range, and the stack depth must be consistent. An image that fails validation is rejected; it never executes.
  - This is the line between "a file we wrote" and "a file someone handed us". Pack images in user caches go through the same check.
- **Writer and reader in `tcl-bytecode`.** They need no compiler and no VM, so the SpecTcl loader, `tcl-db` and the VM can all use them.

## Work list

1. **Byte encoding.** `layout.rs` already assigns byte offsets and resolves labels. Add `encode(&FunctionAsm) -> Vec<u8>`, which emits C Tcl instruction bytes, and `decode`. Check the encoding against `tclsh` 9.0 disassembly for the bytecode-compare corpus. The bytes must be identical, not only the disassembly.
2. **The container.** Add `tclbc::write(&ModuleAsm)` and `tclbc::read(&[u8]) -> Result<ModuleAsm, ImageError>`, with the validation above.
3. **Opcode-set versioning.**
   - Name the current set `tcl-9.0.2`.
   - Record per release which C Tcl opcodes differ, from `tclCompile.h` in each pinned source tree (in the style of items 79 and 202), so a later set is a generated table.
4. **`tbcload` research.**
   - What container `tbcload` reads.
   - Which Tcl releases it supports.
   - Whether a Tcl 9 port exists.
   - Whether an exporter from `tclbc` to `tbcload` is worth building, so that C Tcl can run our precompiled code (and the reverse).

   Record the answer here. The exporter is optional.
5. **Consumers.**
   - Pack images store hook bodies as `tclbc` (305 and 306).
   - The VM can load AOT modules.
   - `tcl-db` may cache compiled procedures by source hash and opcode set, if measurement shows compile time matters.

## Verification

- **Round trip.** `read(write(m)) == m` for every module the bytecode-compare corpus produces, across 8.4–9.1 sources compiled under the 9.0.2 set.
- **Byte identity.** The encoded code bytes match `tclsh` 9.0's for the corpus. A difference is either a known, listed divergence or a bug.
- **Execution.** Running each script from its image gives the same result as compiling it, native and WASM, as a differential test in the conformance runner.
- **Rejection.** Corrupted images are rejected: truncated, bad jump, index out of range, unknown opcode set. A fuzz target goes in `tcl-fuzz`.
- **Measured.** Load time of a hook body from `tclbc` against compiling it, and the size of the hook section in the bundled pack images.

## Risks

- **The extra metadata grows.** `ModuleAsm` carries native-compilation and provenance fields that change often. They live in the versioned metadata section, so the code tables stay stable.
- **One opcode set across eight releases.** Our VM runs the 9.0.2 set for every target release. Exact C Tcl byte identity holds for 9.0 only. For other releases the image is "our bytecode in C Tcl's encoding", and the header says which set it uses.
- **`tbcload` interop may not pay off.** If step 4 finds no maintained Tcl 9 loader, we keep the C Tcl-shaped tables for readability and skip the exporter.
