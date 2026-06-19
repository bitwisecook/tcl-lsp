# Porting S110 (byte-array corruption) to Rust

> **Status:** ✅ landed. The Python implementation landed in PR #656 (on
> `main`); the Rust port lives in `tcl-compiler::shimmer::byte_array` (the
> detector), `tcl-registry` (`BytePayloadSpec` + `CommandSpec::byte_array_payload`
> + `CommandRegistry::byte_array_payload_layouts`), and is wired through
> `compiler_checks::run_all_checks` (`push_shimmer_checks`) and the compiler
> explorer's shimmer view. This document is retained as the port spec for the
> Rust `tcl-compiler::shimmer` subsystem. The owning track is **FE-TYPESHIM** in
> [`../../rust-rewrite.md`](../../rust-rewrite.md). The Python design rationale
> (damage taxonomy, why provenance is separate from the type lattice) lives in
> [`../compiler/byte-array-corruption.md`](../compiler/byte-array-corruption.md)
> (ported alongside this work); the FP contract is in
> [`../compiler/FP.md`](../compiler/FP.md) (FP-SH-09/10).

S110 is a **correctness** shimmer, distinct from the S100/S101/S102
*performance* family. Tcl byte arrays (binary data) and character strings are
different internal representations; when a byte array is forced through a
character-string operation and then written back to a byte sink, every byte
`>= 0x80` is silently re-encoded (latin-1 → UTF-8) or pushed out of `0..255` by
case folding — corrupting the data. The canonical case is the iRules
`*::payload replace` round-trip bug ([F5 K22406348]); the plain-Tcl case is
`binary format` → `string …` → byte sink.

[F5 K22406348]: https://my.f5.com/manage/s/article/K22406348

---

## 1. The Python algorithm (authoritative spec)

Source: `compiler/shimmer.py` on `origin/main`, the `Byte-array corruption
(S110)` section (`_find_byte_array_corruption` + helpers). It is a **forward
dataflow over the SSA graph** tracking *byte provenance* per SSA value.

### 1.1 Provenance lattice

Two states (`_ByteProv`), carried in a `_ByteProvInfo { state, source_range,
source_label, coercion_range, coercion_label }`:

- **`BINARY`** — value currently has a byte-array intrep; safe at a byte sink.
- **`DAMAGED`** — binary-sourced but since coerced to a character string;
  writing it to a byte sink corrupts it.

Join (`_join_prov`): **DAMAGED dominates BINARY** (may-corrupt); the first
non-empty source is kept for the diagnostic. Absence (no entry) = untracked.

### 1.2 Binary sources

- `binary format`, `binary decode`, `encoding convertto` — **dialect-agnostic,
  always on.** Recognised via the registry type hint (`return_type ==
  BYTEARRAY`), see `_cmdsub_returns_bytearray`.
- `*::payload` getters — **iRules-gated.** The getter form is `<proto>::payload`
  with no args, or a first arg that is **not** in `{replace, length, rechunk,
  unchunk}` (`_PAYLOAD_NON_GETTER_SUBS`). Set membership comes from
  `byte_array_payload_commands()` **intersected with the active dialect**
  (`REGISTRY.get(c, dialect) is not None`) — the registry is process-global, so
  a plain-Tcl document that merely names a `*::payload` command must not trip.

### 1.3 Coercion / damage operations (BINARY → DAMAGED or warn-on-spot)

- **Intrinsic corruption — warn immediately** (corrupts with or without a
  write-back):
  - `string toupper|tolower|totitle` (`_CASE_FOLD_STRING_SUBS`) on a binary
    operand → emit S110, mark result DAMAGED.
  - `encoding convertto` on an already-binary value (double-encode) → emit
    S110, then treat the (byte-array) result as BINARY.
- **Round-trip corruption — mark DAMAGED, warn only at the sink** (latin-1
  preserving, tclsh-verified):
  - `string map|replace|range|index|reverse|repeat|trim|trimleft|trimright|cat|insert`
    (`_STRING_VALUE_SUBS`).
  - `format|subst|regsub|join|concat|split` (`_STRING_COERCING_COMMANDS`).
  - `append v …` when target or any appended operand is binary/damaged.
  - `$`/`[` interpolation and `expr` over a binary/damaged operand.
- **The documented fix:** `binary scan $v …` re-binarifies `$v` *in place* →
  clears DAMAGED back to BINARY. (`binary format … $v` does **not** mutate
  `$v`, so it must not clear provenance — only the assigned `set x [binary
  format …]` re-binarifies via the byte-array return type.)

### 1.4 Byte sinks (where round-trip corruption is reported)

- `<proto>::payload replace <offset> <length> <data>` — if the `<data>` arg
  (index 3) is **DAMAGED**, emit S110 (`_payload_replace_data_index`).
- Case-folding / `encoding convertto` are their own sinks (reported on the spot,
  see §1.3).

### 1.5 Transfer functions (per statement, in `cfg.reverse_postorder()` over
executable blocks; phis joined first)

- `IRAssignValue` (`_track_assign_value`): classify the RHS — payload getter /
  byte-array cmdsub → BINARY; `encoding convertto`/case-fold on binary → warn +
  set state; string-value-sub / coercing command over binary → DAMAGED; pure
  `$var` copy → propagate provenance unchanged; interpolation over a
  binary/damaged var → DAMAGED.
- `IRAssignExpr` (`_track_assign_expr`): any binary/damaged use → result
  DAMAGED (coercion label `expr`).
- `IRCall` (`_track_call`): `binary scan $v` → clears `$v` to BINARY; `append`
  → DAMAGED target; `*::payload replace` → sink check (emit if data DAMAGED).

### 1.6 Emission

`_byte_corruption_warning` builds a `ShimmerWarning { code="S110",
from_type=BYTEARRAY, to_type=STRING, in_loop=False, … }` with `related_ranges`
pointing at (a) the binary source and (b) the coercion site. Severity is
**Warning** (`server/features/diagnostics.py`: `"S110": Warning`).

---

## 2. Rust target architecture

The shimmer subsystem is `rust/tcl-compiler/src/shimmer/` (`mod.rs`, `graph.rs`,
`thunking.rs`, `phi.rs`, `use_site.rs`, `expr.rs`, `hints.rs`, `span.rs`). Each
detector is a `find_*` function returning warnings; `mod.rs::find_shimmer_warnings`
is the per-function dispatch and the whole-CU wrappers iterate
`cu.analysable_functions()`.

**Closest templates:** `use_site.rs` (forward walk over `Statement::Call` /
`Statement::AssignValue`, arg parsing, type lookups) for the per-statement shape,
and `thunking.rs` for the `cfg_order(cfg)` + executable-block-gated walk and the
phi-join scaffolding.

**Key types** (verbatim field names matter):
- `ShimmerWarning { span, variable, from_type, to_type, command, in_loop, code,
  message, related: Vec<(Span, String)> }` — note `span` (not `range`) and
  `related` (not `related_ranges`).
- Per-function bundle `FunctionUnit { cfg, ssa, sccp, types, … }`; SSA via
  `SsaFunction.blocks[bn].{phis, statements}`, each `SsaStatement { statement,
  uses: HashMap<String,u32>, defs: HashMap<String,u32> }`; `Phi { name,
  version, incoming: HashMap<String,u32> }`.
- IR statements: `Statement::{AssignValue{name,value,…}, AssignExpr{name,expr},
  Call{command,args,…}, Incr{…}, Barrier{…}}`; `Statement::span()`.
- Helpers: `value_shapes::{is_pure_var_ref, parse_command_substitution}`,
  `naming::normalise_var_name`, `shimmer::span::def_range_map`,
  `sccp::cfg_order`, `shimmer::hints::arg_shimmer_type`.

**Diagnostic glue** is in `compiler_checks.rs` (NOT `analyser/`):
`run_all_checks` loops `cu.analysable_functions()`, calls
`find_shimmer_warnings(...)` / `find_thunking_warnings(...)`, and lowers each via
`Diagnostic::from_shimmer` / `from_thunking` with `shift(fu, …)` to absolutise
spans. Severity is decided **inline per warning family** (no central table):
`from_shimmer` maps S100→Info else Warning.

**Registry** (`rust/tcl-registry/src/`): per-command facts live on `CommandSpec`
/ `Traits` bitflags; iRules `*::payload` specs are under
`rust/tcl-registry/src/commands/irules/`. Dialect membership is queried with
`get_for_dialect(name, dialect)` (the analyser registry only loads the active
dialect into `by_name`).

---

## 3. Port plan

1. **Registry flag.** Add a `byte_array_payload` marker to the `*::payload`
   command specs (HTTP/TCP/UDP/SCTP/DIAMETER/GTP/MQTT) in
   `rust/tcl-registry/src/commands/irules/*payload*` — either a `Traits` bit or
   a `CommandSpec` bool, matching how existing per-command facts are modelled.
   Add a `byte_array_payload_commands(&self) -> …` accessor mirroring the Python
   cache. Confirm `binary`/`encoding` specs carry `return_type == BYTEARRAY` for
   `format`/`decode`/`convertto` (the dialect-agnostic sources).
2. **Detector.** New `rust/tcl-compiler/src/shimmer/byte_array.rs` exposing
   `pub(crate) fn find_byte_array_corruption(cfg, ssa, executable_blocks,
   registry, dialect) -> Vec<ShimmerWarning>`. Port the `_ByteProv` /
   `_ByteProvInfo` lattice, `_join_prov`, the source/sink/coercion sets, the
   `_arg_byte_prov` recursion, and the three transfer functions
   (`_track_assign_value`/`_track_assign_expr`/`_track_call`). Walk
   `cfg_order(cfg)` gated on `executable_blocks`, join phis first. Reuse
   `is_pure_var_ref` / `parse_command_substitution` / `normalise_var_name`;
   build spans from `Statement::span()` / `def_range_map`.
   - **Dialect gating:** the payload-command set must be intersected with the
     active dialect — thread the dialect into the detector (the per-CU wrapper
     has it) and use `registry.get_for_dialect`.
3. **Wire-up.** Call the detector from a whole-CU entry in `mod.rs` (alongside
   `find_shimmer_warnings_for_cu` / a sibling), and add a `from_byte_array` (or
   reuse `from_shimmer`, since `code=="S110"`→Warning falls out of the existing
   match once S110≠S100) emission loop in `compiler_checks.rs::run_all_checks`
   with `shift(fu, …)`. Optionally surface in `tcl-explorer/src/serialise.rs`.
4. **Related spans gap.** `compiler_checks::Diagnostic` has **no related-ranges
   field**, so `ShimmerWarning.related` is dropped at lowering today. S110's
   value comes partly from pointing at both the source and the coercion site —
   either extend `Diagnostic` with related spans (preferred, benefits all
   families) or fold the two locations into the message text. Decide before
   implementing.

---

## 4. Verification contract

Port byte-for-byte against the Python behavioural battery:

- `tests/test_shimmer.py::TestByteArrayCorruption` (18 cases). Fires:
  `kb_http_payload_interpolation_roundtrip`, `tcp_payload_string_replace_roundtrip`,
  `append_then_payload_replace`, `copy_chain_preserves_provenance`,
  `string_map_cmdsub_data_arg`, `plain_tcl_toupper_case_fold`,
  `encoding_convertto_double_encode`, `inline_convertto_at_sink_fires`,
  `inline_case_fold_at_sink_fires`. Silent (false-positive controls):
  `binary_format_does_not_clear_provenance`, `clean_convertto_source_at_sink_silent`,
  `payload_names_outside_irules_dialect_silent`, `clean_payload_writeback_silent`,
  `binary_scan_rebinarify_fix_silent`, `ascii_literal_writeback_silent`,
  `non_binary_string_op_silent`, `direct_binary_source_data_arg_silent`,
  `payload_length_query_not_a_source`.
- `tests/test_fp_sh.py`: FP-SH-09 (`toupper_byte_array_fires` /
  `toupper_plain_string_silent`), FP-SH-10 (`payload_roundtrip_fires` /
  `clean_payload_writeback_silent` / `binary_scan_fix_silent`) — with tclsh
  ground truth.
- Differential: run the Rust S110 against the Python oracle on the iRules
  corpus, and confirm the documented `binary scan` fix yields lossless high-byte
  round-trips against real `tclsh` payload semantics.
