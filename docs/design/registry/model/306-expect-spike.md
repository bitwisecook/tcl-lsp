# Step 306: spike, Expect as data

Status: designed 26 November 2026. Plan item 306, step 6 of
[dialects, libraries and environments](../dialects-and-libraries.md). Builds on
[305](305-spectcl-3.md) (SpecTcl 3, `check`, hook rungs) and
[302](302-libraries.md) (facets). It decides [307](307-libraries-as-data.md).

## Problem

Every per-command fix recompiles Rust. The surfaces are 185k lines of Rust in
`tcl-registry/src/commands/`. The SpecTcl path is too slow to replace them as
it stands: evaluating the Tcl 8.6 equivalence pack takes 697 ms, against 1.3
ms for native `build_default` (`spectcl-compiled-surface-plan.md`, Results).
The cost is evaluation and parsing, not data volume, and nothing yet loads a
surface without parsing.

Expect is the test subject. It is `tcl-registry/src/commands/expect/`: 36
files and 2,742 lines, with 38 `surface` rows and 35 hovers. It has one hook
shape, used 6 times (`case_list: CaseListSpec::EXPECT` with
`AnalyserHookId::Switch`, for example `expect_cmd.rs:172`). It has no folds
and no codegen hooks, so it is hook-light, and it is a library on `tcl` with
an `expect` environment.

Today the packs reach users as source:

| Artefact | How packs ship today |
|---|---|
| VS Code VSIX (universal and six platform targets) | `specs/*.tclspec` copied beside each server binary (`Makefile:373`) |
| VS Code web | copied to `dist/web/specs/` with an `index.json` (`Makefile:353-357`, `copy-web-assets.cjs:41`), mounted by `primeSpecPacks` |
| VS Code WASI server | copied beside the module (`Makefile:402`) |
| JetBrains `.zip` | copied beside each server binary (`Makefile:2438-2440`) |
| Zed, Neovim, Emacs, Helix and other bare-binary clients | none on disk. The server falls back to the eight `include_str!` sources (`tcl-spectcl/src/bundled.rs:113-145`) |
| Sublime package | none. CI asserts it ships no `specs/` (`.github/workflows/ci.yml:3777`) |

## Design

### Compile once per build

`cargo xtask spec-images` compiles every shipped pack to an image in one run:

- the core dialects and libraries (after 307);
- the EDA packs in `specs/`;
- Jim's `core-surfaces/*.tclspec`;
- for the spike, `expect.tclspec`.

The image bytes are platform-independent, so one build's images go into
every artefact above. `--check` (item 119) recompiles and fails if a
checked-in image differs from its source.

### Image format, version 1

All integers are little-endian. All references are `u32` offsets from the
image start, with no pointers. Every section is 8-byte aligned. An image is
valid when the header checks pass, and readers never trust offsets without
bounds checks. A `&[u8]` view is enough, with no allocation for reads.

```text
Header (64 bytes)
  magic        "TCLSPIMG"
  format       u16   = 1            image layout version
  vocabulary   u16   = 3.minor      SpecTcl vocabulary the compiler spoke
  hook_abi     u16                  version of the native hook-ID table
  flags        u16
  source_hash  u64   xxh3 of the pack source bytes (and its includes, in order)
  section_count u32, section table offset u32
  total_len    u32, crc32 u32
Section table: (kind u32, offset u32, len u32, count u32)*
  STRINGS     one string table: (offset u32, len u32)* then UTF-8 bytes; all names, prose
  VERSIONS    interned VersionSets: (axis u32 → STRINGS, range_count u16, ranges (min,max as string ids))
  LIBRARIES   302's Library rows: id, aliases, trains, compatible rows, requires
  ENVIRONMENTS 303's definitions
  SPECS       the spec arena: fixed-size records (SpecRecord) indexed by SpecId u32
  FACETS      (key_id u32, value_off u32)* per facet; first match wins
  NAMES       sorted (name string id, SpecId) for binary search, per library
  RULES       305 `check` expression trees: (op u8, arg u32)* in post-order
  ROLE_TABLES tabulated shape-only hooks: (hook_id, shape_key u64, answer_off)*
  HOOKS       (kind u16, native_id u32 | tcl_body string id, inputs shape)*
```

`SpecRecord` holds offsets into the other sections, with no inline variable
data:

- name;
- `Availability` (library id, window id, dialect-constraint id);
- arity facet;
- options, subcommands and forms (sub-arenas of records);
- roles, effects and returns;
- docs (hover, synopsis, examples as string ids);
- deprecation;
- hook refs.

This is `SpecArena` from the compiled-surface plan's phase 3, laid out as
bytes. Lookup is a binary search on `NAMES`, which yields a `SpecId`. A
`SpecView<'img>` reads fields lazily. `CompiledSurface` (plan phase 3) is
built over views and never copies the data.

**Versioning.** A reader accepts exactly its `format`, and a `vocabulary`
whose major is 3. A mismatch on either, or a `hook_abi` that names an ID the
binary lacks, marks the image *stale*. The pack is then compiled from source
in process: slower, never wrong.

### Two placements

| Pack kind | Placement | Load |
|---|---|---|
| Core dialect and library images (Tcl, Jim, f5-tcl, Tk, tcllib, Expect, iRules, …) | `include_bytes!` in their crate | at registry install, with zero parsing |
| Replaceable vendor packs (`specs/*`) | `specs/NAME.tclimg` beside `NAME.tclspec`, in every artefact that stages `specs/`; also `include_bytes!` in the bare-binary fallback, replacing today's `include_str!` | lazily, through demand loading (`tcl-spectcl/src/demand.rs`, `demand::enable`; the manifest-only path in spec-packs.md) |

The web build fetches `dist/web/specs/*.tclimg` by the same `index.json`.
`include_bytes!` works for `wasm32-unknown-unknown` and `wasm32-wasip1`
because it is a compile-time constant. A byte image of the size measured
below needs no special handling. The Sublime package ships no server and no
packs, so nothing changes for it.

### Replaced and user packs

Each image carries `source_hash`. At load:

1. **A bundled pack replaced on disk.** This means a `specs/` file or
   `TCL_LSP_SPEC_PACK_DIR`. Its source hash is compared with the image's, and
   on a mismatch the image is ignored.
2. **A pack with no current image** is compiled once with the 3 reader (no
   VM). The image is cached under `tcl-userdirs`' cache directory as
   `spec-images/<source_hash>-<format>-<vocabulary>.tclimg` and is reused on
   the next start. Writes are atomic (temp file plus rename). A corrupt cache
   entry fails the header checks and is rebuilt.
3. **`tcl spec compile PACK… [-o DIR] [--check]`** does the same on demand
   for authors and CI.

A 1.x or 2.x pack is upgraded in memory through the shims first. A pack that
computes rows needs the tool evaluator (305), so an image is only produced
when that evaluator is enabled. Otherwise the user gets the upgrade notice.

### Hooks in the image

- **Native hooks** are `(kind, native_id)`. The ID indexes the owning crate's
  hook table, and `hook_abi` versions the table.
- **Shape-only hooks**, which read `kinds`, `argc` and `options` (305's
  context), are tabulated into `ROLE_TABLES` at image build. This replaces
  `BUNDLED_ROLE_TABLES` (`tcl-spectcl/src/bundled_manifest.rs:92`) and
  `cargo xtask gen-bundled-manifests`.
- **Dynamic Tcl hooks keep their source in the image** and are compiled to VM
  bytecode on the first call. Storing bytecode is not possible today:
  - `tcl-bytecode` (`FunctionAsm`, `ModuleAsm`, `lib.rs:165`, `:520`) has no
    serialiser;
  - literals are in-memory `Value`s;
  - nothing versions the instruction set.

  Storing bytecode would take a versioned instruction-set number, a literal
  pool encoded as bytes, a loader that validates jump targets, and a
  differential test that runs the same stored bytecode native and under WASM.
  The bundled packs have one dynamic Tcl hook (`upf.tclspec`'s resolver,
  already tabulated), so compile-at-first-call costs nothing measurable, and
  the spike keeps it.

### The spike itself

1. Author `tcl-lib-expect/expect.tclspec` in SpecTcl 3 from the 36 Rust
   files. `case_list EXPECT` becomes a rung 1 `case_list` row, and the
   `Switch` hook stays `-native Switch`.
2. Add `tcl-spectcl::image::{compile, ImageView}` and `cargo xtask
   spec-images`.
3. `tcl-lib-expect` embeds `expect.tclimg`, and its `Surface` reads the view.
4. Run the equivalence test (305) between the native Expect surface and the
   image, under the `expect` environment and `tcl8.6` with `package require
   Expect`.
5. Measure (below), using the compiled-surface plan's method. Then write the
   decision into 307.

## Work list

1. `rust/tcl-spectcl/src/image/{format.rs, write.rs, view.rs}` with fuzzed
   bounds checks (`cargo fuzz` target on `ImageView::open`).
2. `rust/xtask/src/spec_images.rs` with `--check`, plus a Makefile target
   `xtask-spec-images` added to `xtask-check`.
3. `tcl-lib-expect`: the pack, the image, and `Surface` over the view.
4. Staging: `Makefile` copies `specs/*.tclimg` wherever it copies `*.tclspec`
   (`:353`, `:373`, `:402`, `:2438`), and `copy-web-assets.cjs` adds them to
   `index.json`. `bundled.rs` swaps `include_str!` for `include_bytes!` of the
   images. The source stays embedded for hash checks and the source fallback.
5. `tcl spec compile` in `tcl-cli`.

## Verification

Measure with the compiled-surface plan's harnesses (`ci` profile, 4 vCPU):
`surface_costs` per phase, `dialect_pair_bench.py` for server start and RSS,
and callgrind for attribution.

| Measurement | Baseline | Recorded as |
|---|---|---|
| Expect surface load: native `expect_command_specs()` against image open plus first `CompiledSurface` | native, today | ms, RSS delta |
| Expect lookup: `resolve_spec` known and unknown head | 235 / 20 ns (`tcl8.6`) | ns |
| Server start with **all bundled packs** as images (EDA, Jim, Expect) | 136 ms and 35 MiB manifests; 421 ms for the first EDA document (plan Results) | ms, MiB |
| The Tcl 8.6 equivalence pack as an image | 697 ms evaluated; 1.3 ms native | ms |
| One-command edit to Expect: rebuild set and wall time | today `tcl-registry` and everything above | crates rebuilt, s |
| Image size: Expect, each EDA pack, the 8.6 equivalence pack | source bytes | bytes, compressed in VSIX |
| Bundle growth per artefact: each VSIX, the JetBrains zip, the web bundle, the WASI module | today's artefact sizes | bytes |
| Equivalence | — | zero differences |

**Decision thresholds for 307:**

- **Data** (all libraries as images) if:
  - the 8.6-equivalent image loads within 5× native (≤ 7 ms);
  - Expect's lookups are within +10% of native;
  - server start with every bundled image is no slower than today and uses no
    more memory;
  - a one-command edit rebuilds only the library crate, `tcl-catalogue` and
    the binaries;
  - equivalence is exact.
- **Data except hook-heavy surfaces** if the load and lookup thresholds hold
  but the Tcl core's 1,655 native hook rows make authoring in SpecTcl no
  smaller than the Rust.
- **Native Rust per library crate** (the registry split alone) if image load
  is over 20 ms for the 8.6 surface, or lookup is over +25%, or any
  equivalence difference cannot be fixed in the vocabulary.

**Done when:** the measurements are recorded in this document, and the
decision is written into 307.

## Risks

- **Marketplace size limits.** No size budget is recorded in the repository,
  and the images add bytes to six platform VSIXes and the JetBrains zip.
  Mitigation: record the growth, and drop the `.tclspec` sources from
  artefacts that never let users replace packs (the web bundle) only if the
  owner agrees.
- **Hash drift between source and image.** `--check` in `xtask-check`, and
  the runtime source-hash comparison.
- **A malformed user-cache image.** Bounds checks, the CRC and the fuzz
  target. A failure falls back to source.

## Corrections to the overview

- `spectcl-compiled-surface-plan.md`'s phase 3 (`SpecArena`,
  `CompiledSurface`) is not built. The image is its byte form, so the spike
  builds phase 3 too.
- `bundled.rs:41-43` says the Sublime package stages `specs/`, but CI
  asserts it does not (`ci.yml:3777`). The coordinator's list mentions an
  Emacs bundle, but none exists: `editors/emacs` holds only a README, and
  Emacs uses the bare binary.
- The EDA packs are in `specs/` at the repository root, not in
  `rust/tcl-spectcl/specs/`.

## Owner decisions (26 November 2026)

- **Every artefact ships source and images,** the web bundle included: the `.tclspec` source beside each compiled image.
- **The Tcl core's hover and documentation become an image too.** Its hooks stay native. That is a fixed decision, not one the spike's size breakdown decides.
