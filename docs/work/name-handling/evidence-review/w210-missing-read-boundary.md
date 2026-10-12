# W210 missing-read boundary (temporary finding)

The supplied post-prototype run still returns 0 W210 instead of 1/1/2 for
the three Tcltest negative diagnostics tests, beginning at `tcl8.4`:
`/tmp/w210-selection-tcltest.log`. No build or test was run for this review.

The next narrow boundary is the **reached original metadata join**, not the
physical local-read receipt. Its exact path is:

* `command_binding/declaration_flow.rs`: `Builder::selection` retains a
  `query_dispatch_points` binding only when matching dispatch points exist.
  Points must match dispatch, offset, origin and declaration variable frame.
* `registry_invocation/declaration_flow.rs`: `declaration_invocation_flow`
  first calls `advice.original_declaration_metadata_context(registry, tokens)`.
* `command_binding/declaration_layout.rs`:
  `OriginalDeclarationWordSource::metadata_context` checks purpose, complete
  original vector, origin, registry semantic key and invocation dialect.
  With a retained `lookup_state`, it then requires
  `binding.original_invocation_metadata_at_point(tokens, registry)?` and exact
  equality between the reached point's metadata owner and declaration owner.
  That reached-owner refusal is terminal; the absent-dispatch fix does not
  change this branch. It must stay terminal for an invalid reached owner.

`/tmp/w210-debug-selection.log` establishes `entry=true config=true words=true`
but `selected=None` at the outer `test` selection before the latest prototype.
The later negative run and carrier test establish the continuing failure and
the absence-only fix, respectively, but contain no inner metadata/lifecycle
guard result. Therefore the reached metadata guard above is the next precise
experiment target, **not a runtime-confirmed cause**. Do not loosen it based
only on the earlier outer trace.

One tempting source-read change is excluded by the current owner code:

* `Builder::variable` records `Event::AuthoredRead` for an unqualified original
  reference owned by `entry.owns_source` before calling `retained_local_read`.
* `retained_local_read` requires the exact span/spelling, direct operand owner,
  original layout, matching contexts/activation and no excluded local scope.
  Failure creates `ReadUnavailable`, not loss of the authored occurrence.
* `events_before` marks that node `declared_read_unavailable=true` and clears
  `unavailable_declared_prefix`. `declared_states(..., false, true)` explicitly
  exempts that node from the unknown-command prefix stop.
* `Builder::evaluate` derives `warning_reads` from `node.authored_reads`, not
  `node.reads`. Thus loosening `retained_local_read` cannot repair a warning
  which selection never admits, and would unnecessarily grant read authority.

Next experiment, once the build owner grants a lease: trace only the first
selected `test` in `tcltest_shared_frame_keeps_genuine_undefined_reads_and_data_opaque`
at `tcl8.4`. Record the metadata `Result`, whether `lookup_state` exists,
the reached/declaration metadata-owner equality, and whether the outer
`declaration_invocation_flow` returns `Some`. If metadata succeeds, trace
`scoped_declaration_lifecycle_advice` separately rather than changing this guard.
If outer lifecycle selection succeeds, record `warning_reads["absent"]`
before considering any diagnostic emitter change.

Any repair must retain dynamic demurral, invalid reached-world refusal,
Normal-handler and observer checks, provider/implementation/version matching,
required-absent hooks, original source/frame ownership and result-data opacity.
It should add only conditional original-layout advice to the declaration
warning graph; it must not manufacture entered execution or physical reads.
The real oracle remains
`experiments/name-handling/tcltest-shared-frame/probe.tcl` and its captured
`c-results`: setup defines `prepared`, body genuinely reads missing `absent`,
setup error skips `$unreached`, and expected-result words remain data.
