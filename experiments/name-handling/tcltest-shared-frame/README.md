# Tcltest caller-frame reads

[probe.tcl](probe.tcl) runs the selected distribution's real `init.tcl` and
`tcltest.tcl`. Each C release starts a fresh process. The direct and captured
alias cases read an undefined caller local after setup creates a different
local. The stopped-setup case records whether the body and cleanup entered.
The result-data case retains an unevaluated dollar reference.

| Oracle | Direct / captured alias | Stopped setup | Result data | Capture |
| --- | --- | --- | --- | --- |
| C Tcl 8.4.20 | Expected missing-read error; cleanup sees `1`; `absent` remains absent | Body unentered; cleanup sees `1` | `data_only` remains absent | [Raw output](c-results/8.4.20/stdout) |
| C Tcl 8.5.19 | Same | Same | Same | [Raw output](c-results/8.5.19/stdout) |
| C Tcl 8.6.18 | Same | Same | Same | [Raw output](c-results/8.6.18/stdout) |
| C Tcl 9.0.4 | Same | Same | Same | [Raw output](c-results/9.0.4/stdout) |
| C Tcl 9.1.0 | Same | Same | Same | [Raw output](c-results/9.1.0/stdout) |
| Jim | Unmeasured for these exact C Tcltest inputs | Unmeasured | Unmeasured | Jim's separate [native shared-lifecycle control](../patterns/jim-results-5bac7c9/jim-native/test-shared-lifecycle/record.json) does not answer these inputs |
| F5 | Unmeasured for these exact C Tcltest inputs | Unmeasured | Unmeasured | [Context-specific pending questions](../patterns/f5/questions.json) |

Every C capture records four Tcltest invocations: three passes and one
intentional setup failure. That `FAILED` report is the measured setup-error
control: it records `stop`, body entry `0` and cleanup value `1`. It is not a
failed attempt to load the harness. Process completion is `0`; raw reporting
and error information are retained.

[records.json](c-results/records.json) retains executable, experiment,
`init.tcl`, `tcltest.tcl` and output hashes. Each release directory includes
its exact input, metadata and both output streams. The
[original capture runner](c-results/capture-original.txt) records the code
used for these immutable captures. To create another capture from the same
source-built distributions, choose a new destination:

```sh
python experiments/name-handling/tcltest-shared-frame/capture.py <new-output-directory>
```

The compiler's caller-frame regression controls retain missing reads and
opaque expected results for all five selected C Tcltest providers, including
aliases that capture the test name and description. Their conditional
declaration-flow warnings do not grant entered execution, a physical cell
address or permission to optimise the original wrapper.
