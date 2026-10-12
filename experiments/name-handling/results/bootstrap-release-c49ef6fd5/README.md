# C Tcl library bootstrap observations

Source base: `c49ef6fd5`. These experiments distinguish the explicit native
constructors from the ordinary logical compatibility constructors. Captures
retain executable hashes, matching upstream `init.tcl` and `tcltest.tcl` hashes,
input scripts, stdout, stderr and process status. A startup refusal supplies no
Tcl test verdict.

## Native runtime coverage

`library_bootstrap_oracle.rs` uses `Interp::with_native_core` and runs exact C
reference processes through `tcl-test-support`. Four scoped regression tests
pass in [native-regressions.log](native-regressions.log):

- Real C Tcl 8.4.20 and 8.5.19 `init.tcl` installs `::auto_load`.
- Package-unknown callback argument lists match all five C releases.
- Legacy `string bytelength` matches BMP text, binary NUL and binary data;
  Tcl 9.0/9.1 rejects the removed subcommand.
- The Tcl 9.1 `::tcl::InitAutoPath` handler preserves existing order and
  duplicates, appends unique paths, consults `TCLLIBPATH` only when absent,
  expands `~`/`~/`, observes read/write traces and preserves safe-interpreter
  paths without parsing an existing malformed list.

The byte-count helper reads the original native string representation. It does
not reconstruct UTF encoding from a Unicode projection. Escaped NUL is a
separate source-producer mismatch: the retained native 8.4/8.5 probes report
`1`, while C reports `2`; binary NUL reports `2` in both.

## Real Tcltest startup

The C controls load matching Tcltest and pass the one retained namespace,
variable, procedure and alias test. Native runtime startup reaches these
blockers before that test can run:

| Release | Reached blocker |
| --- | --- |
| 8.4.20 | Operational refusal: actual indexed variable receiver handler policy unselected |
| 8.5.19 | Guest error: `path add` cannot resolve the Tcl module-path ensemble member |
| 8.6.18 | Init returns an empty error; the original host refusal needs the checked library-completion boundary |
| 9.0.4 | Operational refusal: original local alias table receipt handler policy unselected |
| 9.1.0 | Operational refusal: original local alias table receipt handler policy unselected |

[runtime-native-records.json](runtime-native-records.json) identifies the
immutable native runner and its source receipt in
[the native lifecycle capture](../runtime-lifecycle-fix/native-runner/).
These are startup coverage gaps, not failing upstream naming-test counts.

[records.json](records.json) retains the logical-adapter comparison separately.
Its answers do not establish native implementation conformance.
[vm-native-records.json](vm-native-records.json) retains the initial native VM
attempt, which lacked explicit library publication; none reached Tcltest.
[vm-native-ready-records.json](vm-native-ready-records.json) captures the
release-selectable runner after explicit native library publication. Tcl
8.4/8.5 initialise, then refuse compiler preflight before loading Tcltest; Tcl
8.6/9.0/9.1 refuse that preflight during init. None executes a naming test.

## Remaining work

Provide the actual native compiler/handler policy prerequisites, repair the
Tcl 8.5 module-path ensemble dispatch, correct escaped-NUL production at its
owner, then rerun every real release suite. Named-user tilde lookup needs an
independent host capability; the shared path helper refuses that unavailable
operation instead of guessing a home or dropping a valid path.

The standalone runtime build blocker was unbounded generic callback-type
recursion during variable-link retirement. `RetiredVariableParts::finish`
now enters an erased callback boundary for recursive retirement. The initial
compiler refusal remains in [baseline-build-refusal.log](baseline-build-refusal.log).
