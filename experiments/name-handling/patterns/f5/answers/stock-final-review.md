# Original stock Tcl final errors

The sealed `nh20261012stock2` receipts support original final-operation answers for `factory-body-near-miss` and `string-prefix-near-miss` on stock Tcl 8.4.13 and 8.5.13. Each complete error stack identifies the outer authored final command at original line 4; the exact source definition prefix completed before that command began.

The factory near miss invokes `::nh::red`, then fails on the retained body `return $name` because that procedure has no local `name`. The string prefix near miss enters its authored final `eval`, whose inner script splits the spaced command name and fails on `::nh::two`. These code 1 results are measured final-operation errors; they are not setup failures or unsupported host features.

[The derived review](nh20261012stockfinal1/README.md) retains four measurements, four narrow candidate proofs and a hash-bearing independent source/stack review validated by the strict review helper. Every derived record retains actual capture run ID `nh20261012stock2`, its original invocation and raw output. No new execution is claimed and the sealed stock2 run is unchanged. Wrapper setup is an empty successful no-op; fresh interpreter processes supply isolation, while source and complete stacks establish prefix completion.
