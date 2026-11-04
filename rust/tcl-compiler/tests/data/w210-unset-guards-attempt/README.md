# #2371 failed narrow candidate

This is a failed experiment, not an implemented fix. Product source was
`6e6d7b70ac7f20fb72afe1d1385a615f991cca15` plus the archived diagnostic patch.
The exact candidate/test/input hashes are in `freeze.json`. Every frozen file
matched before and after compilation and execution.

The emitted production-pipeline regression failed its first original-case
assertion under Tcl 8.4: the analyser emitted no W210. The other 44 case/profile
assertions were not reached. The bounded existing RBS trace records
`dynamic_writes=true` and `physical_missing=0`. The physical/source gate was
not weakened. The code and failing test are preserved in a named local stash;
the archived test text is not discovered by CI.

The native matrix separately has 54 passing case/interpreter expectations for
C Tcl 8.4–9.1 and the original Jim build. `latest-jim-oracles.json` independently
reruns all nine cases with current upstream Jim `0.84-10-g82fb4d0`, after syncing
the updated shared reference manifest. Native errors are captured completions,
not a product diagnostic success.

The proposed use of an attempted-read SSA version does not repair the current
source/read admission gap, so #2371 remains unresolved. The prior separate
conditional-producer false positive remains recorded in the main fixture.
