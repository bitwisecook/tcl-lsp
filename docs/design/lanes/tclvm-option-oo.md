# TclVM option/TclOO lane

## Goal

Close #1609 and only the TclOO / `info object` / `info class` slice of #1607.
The registry remains the source of truth for second-level TclOO operations,
property options and kinds, and their release gates. `tcl-cmd-core` remains
the source of truth for exact-first unique-prefix resolution and
`must be ..., or ...` rendering. Both native engines consume that shared
answer.

The lane does not include #1594 or the remaining option-message sweep in
#1607.

## Decisions

- `tcl-registry::commands::tcl::info_oo_subcommands` projects the existing
  `SubSubCommand` rows for one `TclVersion`. It does not declare another name
  table.
- The projection uses the resolved document context for the release and the
  existing `SubCommand::available_sub_subcommands` inheritance rule. The
  9.0-only `creationid`, `definitionnamespace`, and `properties` rows therefore
  disappear before prefix uniqueness and choice rendering are decided.
- `InfoOoSubcommands::resolve` delegates matching and error construction to
  `tcl_cmd_core::ensemble::{resolve_subcommand, unknown_subcommand_message}`.
- The generic registry `SubCommand` second-level resolver now delegates its
  prefix scan to the same `tcl-cmd-core` owner too.
- The `properties` second-level rows carry their `-all`, `-readable`, and
  `-writable` `OptionSpec`s. The registry's typed resolver projects those rows
  and delegates to `prefix::OptionTable`; neither engine keeps another option
  array.
- The Tcl 9.0 configurable-property `-get`, `-kind`, and `-set` options and its
  `readable`, `readwrite`, and `writable` kinds live beside the `oo::define`
  registry metadata. Typed registry resolvers delegate matching and miss
  rendering to `prefix::OptionTable`.
- `tcl-vm` and `runtime/rust` pass only their selected `TclVersion`. Neither
  engine carries a copied subcommand/option/kind array or hand-renders those
  choice lists.

## Oracle

The Tcl 9 expectations were measured with
`/home/jimd/src/tcl9.0.4/unix/tclsh` and
`LD_LIBRARY_PATH=/home/jimd/src/tcl9.0.4/unix`. The Tcl 8.6 absence cases were
measured with `/usr/bin/tclsh8.6` (8.6.17).

Pinned cases:

- `info object cl $o` resolves to `class` on both releases.
- `info object bogus $o` uses the ensemble Oxford-comma list.
- Tcl 8.6 rejects object `creationid` and `properties`, and class
  `definitionnamespace` and `properties`, with lists that omit those rows.
- `info class def C nope` resolves to `definition` on 8.6 but is ambiguous on
  9.0 because `definitionnamespace` has entered the table.
- `property x -` is ambiguous among `-get`, `-kind`, and `-set`; `property x
  -kind r` is ambiguous between `readable` and `readwrite`.
- `info {object,class} properties ... -a` accepts the unique `-all` prefix;
  `-` and the empty word use Tcl's `ambiguous option` wording.

## Site inventory

| Site | Status | Result |
|---|---|---|
| `rust/tcl-registry/src/commands/tcl/info_.rs` | Done | Release-filtered shared projection and owner tests |
| `rust/tcl-registry/src/commands/tcl/oo_define.rs` | Done | Property option/kind registry tables, typed resolvers, and owner tests |
| `rust/tcl-registry/src/spec.rs` | Done | Second-level prefix scan routed through `tcl-cmd-core::ensemble` |
| `rust/tcl-vm/src/cmd_oo.rs` | Done | Shared projections and property resolvers consumed; copied scans/rendering removed |
| `runtime/rust/src/cmd_oo.rs` | Done | Copied tables, scans, and rendering removed |
| `rust/tcl-vm/tests/cross_version_info_surface_e2e.rs` | Done | 8.6/9.0 engine and live-oracle vectors |
| `runtime/rust/src/cmd_oo.rs` tests | Done | Both releases, prefix uniqueness, and exact messages |
| Remaining #1607 families | Out of scope | Inventory at hand-off after the focused gates |

## Behavioural deltas

- `tcl-vm` now accepts unique prefixes such as `info object cl`.
- Both engines hide the Tcl 9.0-only TclOO introspection operations at 8.6.
- Both engines enumerate every available registry operation in Tcl ensemble
  order and retain the comma before `or`.
- Prefix ambiguity is release-sensitive; notably, `info class def` changes at
  9.0.
- TclOO property-definition and property-introspection options now accept
  unique prefixes and distinguish `bad` from `ambiguous`, including the empty
  word.

## Exact remaining #1607 inventory

After this bounded lane, the named population in #1607 that remains is:

- `rust/tcl-vm/src/interp.rs` — the separate `interp debug` noun/prefix bug;
- `rust/tcl-vm/src/cmd_info.rs` — `canonical_info_sub`;
- `rust/tcl-vm/src/cmd_file.rs` — `canonical_file_sub`;
- `rust/tcl-vm/src/cmd_string.rs` — `resolve_string_sub`;
- the issue's broad, deliberately approximate cross-engine message sweep
  (described there as “~78”), excluding the TclOO property and
  `info object`/`info class` sites completed here.

The fourth named hand-rolled loop from #1607, formerly
`rust/tcl-vm/src/cmd_oo.rs::match_prop_option`, is removed. This lane does not
claim or re-count the issue's intentionally approximate full-repository sweep;
that is separate per-ensemble work.

## Open uncertainties

None within #1609. Unimplemented TclOO introspection bodies in `tcl-vm` remain
separate from this dispatch-table ownership correction.
