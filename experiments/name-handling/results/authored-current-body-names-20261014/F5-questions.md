# BIG-IP current-frame evaluated body names

Run the retained `oracles-isolated/current-frame-bodies.tcl` probe in an iRule context that supports the required commands. Retain the BIG-IP release, event/context, available commands, exact script, raw output and error details. Determine whether brace-quoted `eval` conditions read the caller's local `x`, whether `uplevel 0` writes the same local `x`, and whether empty and literal `$literal` variable names remain distinct valid names inside the evaluated body. Record unsupported commands or rejected script forms explicitly.

Retain answers and raw observations in this branch. No hosted result follows from the C Tcl or Jim results.
