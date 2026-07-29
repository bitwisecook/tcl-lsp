# A direct call into a private `::tcl::` implementation namespace: W143,
# with a quick fix rewriting it to the public `dict create` ensemble call.
set d [::tcl::dict::create a 1]

# Private machinery with no public spelling: W143, but no quick fix — a
# `clock GetSystemTimeZone` rewrite would not run.
::tcl::clock::GetSystemTimeZone

# The public ensemble call is the documented way, and is never flagged.
set e [dict create a 1]

# tcllib publishes public packages inside `::tcl::chan`, so these are real,
# documented commands rather than Tcl internals.
package require tcl::chan::memchan
set c [tcl::chan::memchan]

# Tcl's own public `tcl::`-rooted namespaces are never flagged either.
set n [tcl::mathop::+ 1 2]
