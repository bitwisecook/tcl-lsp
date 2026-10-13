# Source variable read frames

The exact direct C Tcl probe distinguishes root, current-procedure and actual-caller reads of the same bare variable under `uplevel #0`, `uplevel 0` and `uplevel 1`. C8.4.20, C8.5.19, C8.6.18, C9.0.4 and C9.1.0 each returned `ROOT`, `PROCEDURE` and `CALLER`, with exit0 and no stderr.

A Logical source consumer can select the root and current declaration tables. A nonzero relative level requires an independently retained caller activation; the executed caller value in this probe does not provide one to an unentered source body. Binary, probe and matching upstream initialization-file hashes are recorded in `results.json`. No Tcltest is used. These receipts establish interpreter observations; they do not establish Rust consumer, JimTcl or F5 acceptance.
