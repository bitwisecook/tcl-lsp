# Actual all.tcl orchestration result

Actual C completes the full var file with 90 passes / 20 real skips / 0 failures. Native reaches actual real init, real Tcltest and runAllTests, then exits 1 with `invalid command name "ParseFormatArgs"` during the orchestration start timestamp, before any started test ID or suite summary. This is a script error, separately recorded from Native host-policy refusal. No Native all.tcl or var pass claim follows from this probe.

[C record](../c/all-tcl-var/record.json), [C stdout](../c/all-tcl-var/stdout.txt), [Native record](../runtime/all-tcl-var/record.json), [Native stdout](../runtime/all-tcl-var/stdout.txt), [Native stderr](../runtime/all-tcl-var/stderr.txt); [source/question](../questions/all-tcl.md).

The independent real-init startup controls both reach exact NH_INIT, exit 0 and empty stderr: [C startup](../c/startup/record.json), [Native startup](../runtime/startup/record.json). Native direct suite measurements use unchanged real Tcltest and files with the real upstream in-process frame configuration, as documented per suite; they do not establish all.tcl completion.
