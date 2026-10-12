# C8.6 stock clock map order proposal

The original [C8.6 capture](../../../../experiments/name-handling/clock-bootstrap-epoch/registration-repair/c-init.stdout.txt)
reports `add clicks format microseconds milliseconds scan seconds` both before
and after authentic init. The [runtime capture](../../../../experiments/name-handling/clock-bootstrap-epoch/registration-repair/runtime-inventory.stdout.txt)
reports `clicks seconds milliseconds microseconds add format scan`.
Membership, deferred targets and prefix flags match; ordered map bytes differ.

The descriptor owner is `rust/tcl-registry/src/commands/tcl/clock_.rs`, specifically
`InvocationDialect::clock_bootstrap_ensemble`. Its C8.6-only
`NativeEnsembleBootstrap` currently authors the namespace and deferred members,
but not the complete ordered map. The consuming owner is
`runtime/rust/src/interp/stock_ensembles.rs::register_stock_ensemble`: it appends
callback entries in callback-roster order, then appends deferred entries.

Extend `NativeEnsembleBootstrap` with an ordered `map_members` slice containing
the seven names in the measured C8.6 order. Expose this through a generic
release-selected `ensemble_bootstrap_map_members(family)` accessor beside the
existing deferred-member accessor; the clock descriptor owns the actual data.
Unselected families/releases retain their current construction path.

Keep worker registration separate. After the existing callback publication and
deferred-target construction, `register_stock_ensemble` should assemble the
initial `EnsembleConfig.map` by consuming those already constructed entries in
the selected authored order. Require every authored member exactly once and no
extra entries; a malformed descriptor must not silently discard a mapping.
Preserve existing publication/currency checks and reached installer settlement.
This is constructor-only metadata assembly, not a transformation of live maps.

The deferred add/format/scan rows remain mapped without callback tokens.
The four reading tokens retain their independently authored Present compiler
descriptors. Map order supplies neither hook presence nor opcode acceptance,
handler effects, procedure identity, or execution authority. No Unknown-to-Absent
fallback or additional support-worker registration follows from this change.

A useful focused regression compares the C8.6 pre-init map serialisation with
the literal ordered map in the retained C oracle, independently of the new
descriptor slice. It also checks that the three deferred command tokens are
absent. Publish their three procedures using the existing scalar birth control;
then assert that the ordered map is unchanged and known compiler/resolver
epochs are preserved. Existing seconds-replacement coverage must still advance
only compiler epoch. The other release paths should remain unselected.

This bounded repair would fix exposed initial map order only. Keeping callback
registration separate leaves the known reading-worker registration-order gap
open. Neither ordered dict output nor successful scalar publication establishes
physical hash buckets, table enumeration order, allocator chronology or the
native compiled-foreach ownership protocol. Those require separate original
registration/table observations and support; no blanket sorting of live maps
is proposed. F5 and unmeasured releases receive no answer from this proposal.
