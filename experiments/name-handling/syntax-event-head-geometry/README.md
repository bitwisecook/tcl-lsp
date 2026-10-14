# Written event-head geometry

The input defines ordinary Tcl commands named `when`, `:when`, and `named::when`. It queries and invokes written heads to distinguish a root command, a lone colon, an interior qualifier, and an empty tail. These observations test name geometry, not F5 event registration or execution.

C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 resolve two-or-more leading colons to the root `when`. The syntax-only default event boundary recognizer delegates this written geometry to `canonical_written_command`; resolved semantic callers supply their own predicate. Each provider retains its own original output and dependency record. Jim results remain independent: `named:::when` is unresolved in this capture, while C resolves it to `::named::when`. All providers agree on the default recognizer's root-command versus lone-colon/qualified/empty-tail partition.

No `tcltest` package is used. Each C process uses its matching original `init.tcl` library directory.
