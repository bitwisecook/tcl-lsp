# Global body frame scope

Original source-built C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and current retained Jim were measured with the scripts and original release libraries identified in each input receipt. No tcltest or hosted F5 interpreter was used. Interpreter/library/script hashes before and after every capture agree.

All six providers leave the suspended procedure's `x` equal to `LOCAL` for a direct global body write and a newly invoked procedure callback. Nested `uplevel #1` and `uplevel 1` in the selected global body fail with a bad-level result. A namespace alias to a procedure variable is refused. These are narrow observed name/frame results; they do not certify arbitrary external command or observer closure.

All five C providers install the variable trace through `trace add variable` and retain caller-local `x == LOCAL` after the global write invokes it. Inside the callback `#1` names the callback's own frame. The retained Jim build has no `trace` command; its capture records that unavailability rather than a trace result.

The twelve isolated captures exit successfully. The six combined captures remain under `oracle-results`; the C 9.x captures abort at removed legacy `trace variable`, and Jim aborts at unavailable `trace`. Their earlier scope outputs are retained individually, without claiming the entire combined capture passed. Every result includes argv, exit status, stdout/stderr, and the measured input hashes.

The source-analysis obligation is separate: a selected global coordinate can locate direct name operands without closing unknown body continuation, aliases or external observers. Source consumers must retain those uncertainties until their genuine owner establishes the needed scope/effect property.
