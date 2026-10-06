# Native ensemble configuration compiler epochs

The private-header probe reports the original `info` command compiler attachment
and interpreter compiler counter around successful, repeated, and rejected
configuration updates on Tcl 8.4.20 through 9.1.0. The manifest pins the actual
source, headers, libraries, and executable observations.

A completed configuration transaction calls the subcommand and mapping setters,
and Tcl 8.6+ also calls the parameter setter. Each setter checks the actual
configuration token's compiler hook. Unknown-handler and prefix setters do not
advance the compiler counter. Updating to the same value still runs the native
transaction. Failed option parsing performs no setter transaction. Tcl 8.4 has
no native ensemble configuration operation.

Run `python3 run.py --tcl-root /path/to/tcl-source-parent --output /tmp/ensemble-proof`
with a new output directory. Compare counters relative to the initial row in the
same interpreter; bootstrap counters are release-specific.
