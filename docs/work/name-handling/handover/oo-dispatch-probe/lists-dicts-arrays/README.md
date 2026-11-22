# List, dict and array fixtures

`l*.tcl` run under tclsh, the VM and the runtime (`engines.out`); `folds/g*.tcl` are constant list and dict operations given to the optimiser; `analyser.out` holds optimiser rewrites and tcl9.0/tcl8.6 diagnostics for both sets. `runtime-dict-with-stale-puts.tcl` reproduces the runtime printing an empty line for `puts $d` after `dict with` changes `d` (tclsh prints `a 10 b 2`).
