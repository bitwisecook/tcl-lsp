# Byte-array corruption detection (S110)

`S110` flags binary data that is forced through character-string semantics in a
way that corrupts it. It is a **correctness** check, distinct from the
`S100`/`S101`/`S102` [shimmer](../../GLOSSARY.md#shimmer) *performance* family.
This note records (a) the damage taxonomy that drives the check and (b) why
byte-array provenance is tracked by a dedicated forward dataflow rather than in
the type lattice.

## Why a byte array gets damaged

A Tcl value is logically a string, but caches one internal representation
(intrep) at a time. A byte array stores raw bytes; a character string stores
Unicode code points. When a byte array's string representation is generated,
each byte `0x00`–`0xFF` becomes the latin-1 character `U+0000`–`U+00FF`. Two
things can then go wrong:

1. **Intrinsic corruption** — the operation mutates the bytes in its own
   result, with no write-back required. Case folding maps a byte to a
   different code point (`0xC3` → `0xE3`) or out of byte range entirely
   (`0xFF` → `U+0178`); `encoding convertto` re-encodes the latin-1 characters
   as UTF-8 (double-encoding). The damage is unconditional.
2. **Round-trip corruption** — the operation preserves the bytes as latin-1
   characters, so the *result* is byte-clean, but it is now a **string**.
   Writing that string to a byte sink (`<proto>::payload replace`, or any
   UTF-8 channel) re-encodes every byte `≥ 0x80`. The damage lands at the
   write-back.

### tclsh ground truth

Applying each operation to the pure high-byte array `80 c3 ff fe` (no ASCII
letters, so any change to a preserved byte is real corruption), then
re-binarifying the result — identical on Tcl 8.6.14 and 9.0.3:

| Operation | Result rep | Category |
|---|---|---|
| `string toupper` | `80 c3 78 de` (bytes changed) | **intrinsic** (case fold) |
| `string tolower` | `80 e3 ff fe` (bytes changed) | **intrinsic** (case fold) |
| `string totitle` | `80 e3 ff fe` (bytes changed) | **intrinsic** (case fold) |
| `encoding convertto utf-8` | `c2 80 c3 83 c3 bf c3 be` | **intrinsic** (re-encode) |
| `string range` / `index` / `reverse` | keeps the byte-array intrep | **transparent** |
| `string trim` / `trimleft` / `trimright` | character string (an effective trim builds one — `StringTrimCmd` → `Tcl_NewStringObj` in 8.6 and 9.0; only a no-op trim keeps the object) | round-trip (coerce) |
| `string map` / `replace` / `insert` / `cat` / `repeat` | character string | round-trip (coerce) |
| `format %s` / `join` / `concat` / `split` / `regsub` / `subst` | character string | round-trip (coerce) |
| interpolation `"$x"` / `append` | character string | round-trip (coerce) |

The **transparent** row is the crucial precision distinction. `string range`,
`index`, `reverse`, `trim`, `trimleft`, and `trimright` return a value that
still carries the byte-array internal representation on both tclsh 8.6 and 9.0
(confirmed with `tcl::unsupported::representation` and a round-trip through a
`-translation binary` sink), so `string range $payload …` written back with
`*::payload replace` is byte-exact and must **not** raise S110. `string cat`
and `string repeat` keep the intrep only in their single-operand no-op form;
their concatenating forms coerce (and differ across versions), so they are
classified as coercing to stay sound.

On Tcl 9 the round-trip back through `binary scan` of a case-folded value
*raises* (`expected byte sequence but character … was 'Ŷ' (U+000178)`); on
Tcl 8 it silently truncates. Either way the byte array is gone.

This is the canonical iRules payload-rewrite bug
([F5 KB K22406348](https://my.f5.com/manage/s/article/K22406348)); the
`HTTP::payload replace` man page documents the same hazard and the
`binary scan … c* throwaway` fix.

## How the check maps to the taxonomy

`tcl_compiler::shimmer::byte_array::find_byte_array_warnings` runs a small
forward dataflow over the SSA graph, tracking a two-state provenance per value:

- **BINARY** — currently a byte array (safe at a byte sink). Sources:
  `binary format` / `binary decode` / `encoding convertto` return types (read
  from the registry, the same metadata the type lattice uses), and `*::payload`
  getters (registry flag `byte_array_payload`, dialect-gated).
- **DAMAGED** — a binary-sourced value since coerced to a character string.

Which operation does which is **registry data**, not a hardcoded command list
in the compiler: every value-transforming command / subcommand carries a
[`ByteArrayEffect`](../../../rust/tcl-registry/src/byte_array_effect.rs) —
`None`, `Transparent`, `Coerces`, `CaseFolds`, `Encodes`, or
`Rebinarifies { value_arg }` — on its `CommandSpec` (whole commands like
`format` / `join`) or `SubCommand` (`string`'s subcommands). The S110 pass
reads that classification through `resolve_cmd_effect(registry, cmd, args)`,
which resolves the subcommand and returns a `CmdEffect` carrying the effect,
its display label, whether the form returns a byte array, and the first
operand index — and never matches a command by name. A `Transparent` op
propagates the operand's provenance unchanged (a binary operand stays binary),
which is what makes `string range $payload` byte-exact instead of a false
positive.

It emits S110 in two places, matching the two damage mechanisms:

- **Intrinsic** (`ByteArrayEffect::CaseFolds` and `ByteArrayEffect::Encodes` —
  the latter is `encoding convertto`): warn at the transform — the bytes are
  already corrupt. A `CaseFolds` result is left DAMAGED; an `Encodes` result
  is a fresh byte array, so it is recorded BINARY after the warning.
- **Round-trip** (`ByteArrayEffect::Coerces`, interpolation, `append`, `expr`):
  mark the value DAMAGED and warn only when it
  reaches a `<proto>::payload replace` sink. A `binary scan $v …` between the
  coercion and the sink re-binarifies `v` *in place* and clears DAMAGED (the
  documented fix — declared as `Rebinarifies { value_arg: 0 }` on `binary
  scan` and `{ value_arg: 1 }` on `binary encode`). `binary format … $v` does
  **not** clear it — it returns a new value and does not mutate `$v`; only the
  assigned form `set x [binary format …]` re-binarifies (via the byte-array
  return type). The intrinsic checks also apply to the **inline sink form** —
  `arg_byte_prov` refuses the byte-array shortcut for an `Encodes` form, so
  `<proto>::payload replace … [encoding convertto utf-8 $payload]` is DAMAGED
  rather than clean.

The `<proto>::payload replace` sink's `<data>` operand is **not** at a fixed
argument index — `replace OFFSET LENGTH DATA` (TCP/HTTP/…, data at index 3),
`replace DATA …` (MQTT/DIAMETER, index 1), and GTP's `replace ('-message'
MESSAGE)? OFFSET COUNT NEW_VALUE` (index 3, shifted to 5 by the optional flag)
all differ. The data position is therefore declared per command in the registry
(`BytePayloadSpec` on `CommandSpec::byte_array_payload`, with fields
`replace_data_index` and `message_flag_shift`) and resolved per call site by
`BytePayloadSpec::replace_data_arg(args)`; the pass obtains the per-command
layouts from `CommandRegistry::byte_array_payload_layouts()`, so it stays
correct for new payload commands without editing `shimmer/`.

## Why provenance, not the type lattice

The type lattice already has `TclType.BYTEARRAY` and a `SHIMMERED(from, to)`
state, so an obvious question is whether byte-corruption should be modelled
there. It should not.

1. **Type vs provenance are different questions.** The lattice answers *what
   intrep does this value have at this point* — a per-program-point property
   computed by join. Corruption is a *flow* property: did a value travel from a
   binary source, through a string coercion, to a byte sink. `[string range
   $ba 0 5]` **is** a `STRING` (the type is unambiguous); only its *origin* is
   binary. The lattice correctly types it `STRING` and necessarily drops the
   origin.
2. **`SHIMMERED` is load-bearing and means something else.** `SHIMMERED(A, B)`
   means a value's intrep is *ambiguous* (A on one path, B on another, or
   oscillating across a loop) and it drives the `S100`/`S101`/`S102`
   performance warnings. A byte-derived string is not ambiguous — it is
   definitively a string. Re-tagging it `SHIMMERED(BYTEARRAY, STRING)` would be
   semantically wrong and would fire spurious *performance* shimmer warnings on
   every binary→string operation.
3. **The damage condition needs reachability, not a join.** Round-trip
   corruption is conditional on reaching a byte sink without an intervening
   re-binarification. A join lattice cannot express "this value reaches
   `payload replace` un-re-binarified"; a forward taint-style dataflow can. The
   codebase already separates [taint](../../GLOSSARY.md#taint-analysis) from
   type inference for exactly this reason, and byte-provenance is a specialised
   taint ("binary-origin data").
4. **The fix is a side-effect, not a redefinition.** `binary scan $v c* -`
   re-binarifies `v` *in place* — it creates no new SSA version. An
   SSA-versioned type lattice cannot model an intrep reset that has no def; the
   provenance pass tracks it as a flow side-effect.

The pass therefore *consumes* the lattice's binary-source signal (BYTEARRAY
return types) but keeps the DAMAGED / round-trip reasoning in its own dataflow.

### Why `*::payload` getters are not typed BYTEARRAY in the lattice

A payload command's `CommandSpec` carries no `return_type` at all, so the type
lattice leaves the value unknown rather than BYTEARRAY. Two reasons: (a) the
getter form coexists with subcommands that return other types (`TCP::payload`
returns data, `TCP::payload length` an integer), and a single spec-level
`return_type` cannot express both; and (b) typing every payload value
BYTEARRAY globally risks new `S100`/`S101` performance-shimmer false positives
with an uncertain blast radius. The S110 pass recognises payload sources from
`CommandSpec::byte_array_payload` instead (`is_payload_getter`, which also
requires the call to be the getter form rather than `replace` / `length`),
which keeps the reasoning contained.

## Dialect scoping

`*::payload` source/sink recognition is iRules-specific. The command registry
is process-global, so once any iRules document loads the f5-irules pack the
payload commands stay registered for the rest of the session. The gate is at
the call site rather than in the pass: `run_all_checks`
(`rust/tcl-compiler/src/compiler_checks.rs`) builds the `payload_layouts` map
from `registry.byte_array_payload_layouts()` only when
`is_irules_dialect(dialect)` holds, and hands an **empty** map otherwise. With
no layouts, `is_payload_getter` and the sink arm both answer `false`, so a
plain-Tcl document that merely names a `*::payload` command never trips S110.
The dialect-agnostic binary sources (`binary format`, `binary decode`,
`encoding convertto`) stay enabled everywhere.

## Pointers

- `rust/tcl-compiler/src/shimmer/byte_array.rs` — `find_byte_array_warnings`,
  the `ByteProv` / `ByteProvInfo` lattice, `join_prov`, `resolve_cmd_effect`,
  `is_payload_getter`, `arg_byte_prov`, and the `track_assign_value` /
  `track_assign_expr` / `track_call` transfer functions
- `rust/tcl-compiler/src/shimmer/mod.rs` — `find_byte_array_warnings` (per-function)
  and `find_byte_array_warnings_for_cu` (whole compilation unit)
- `rust/tcl-registry/src/byte_array_effect.rs` — `ByteArrayEffect` and
  `BytePayloadSpec`'s `replace_data_arg` / `is_getter_call`
- `rust/tcl-registry/src/spec.rs` — `BytePayloadSpec`, `CommandSpec::byte_array_payload`
- `rust/tcl-registry/src/registry.rs` — `CommandRegistry::byte_array_payload_layouts`
- `rust/tcl-registry/src/commands/irules/*__payload.rs` — `byte_array_payload: Some(BytePayloadSpec::DEFAULT)`
  (default index-3 layout) or an explicit `Some(BytePayloadSpec { … })`
- `rust/tcl-compiler/src/compiler_checks.rs` — `run_all_checks`, where the
  dialect gate builds the payload-layout map and the warnings are lifted to
  `Diagnostic::from_shimmer`

Shared:

- `rust/tcl-compiler/src/analyser/diagnostics/fp/sh.rs` — the paired
  must-fire / must-stay-silent regression tests for both damage kinds
- `docs/kcs/features/kcs-feature-byte-array-corruption.md` — user-facing note
