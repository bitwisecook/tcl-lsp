# Actual candidate build failed before controls

The granted offline `tcl-vm --lib --no-run -j2` rebuild exited 101: the new cfg(test) public namespace control omitted the `ROOT_NS` import, producing two E0425 errors. No control executed and no semantic repair acceptance is claimed. The actual compiler diagnostics remain in [vm-build.jsonl](vm-build.jsonl); [vm-build.stderr.txt](vm-build.stderr.txt) and [execution record](vm-execution-record.json) retain the complete build attempt.

The candidate production owner and all 4,943 broadly snapshotted Rust source/manifests/toolchain files remained byte-identical through this failed build. [Before](vm-source-before.json) and [after](vm-source-after.json) include the actual current uncommitted dependency changes; this is a broad currency guard including unlinked runtime files, not an asserted compiled closure or published-HEAD identity. No source, existing caller, fixture, arguments or assertions changed during the lease.

A bounded cfg(test)-only `ROOT_NS` import correction has been reported for a coordinated source window. A later attempt needs a fresh receipt destination and truthful rebuild; the unchanged original constructor control still must execute first, followed by the new public namespace control. This failed log and candidate preimage/hashes remain immutable.
