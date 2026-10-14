# BIG-IP dictionary names and missing input

Run the retained `oracles-isolated/known-missing.tcl` probe in a supported iRule execution context. Record BIG-IP version, event/context, available `dict` commands, exact original script, output and error details. For both the empty dictionary variable name and literal `$missing`, determine whether missing input returns an error before the body executes, whether the variable remains absent, and whether the empty-body form differs from the body that sets `::body_ran`.

Retain answer documents and raw observations in this branch. No hosted result is inferred from the C Tcl or Jim captures.
