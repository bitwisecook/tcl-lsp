# W103: open with a variable argument — pipeline risk
# Source: SpiceGenTcl/src/generalClasses.tcl:2233
#
# If the filename variable starts with "|", open will execute it as a
# command pipeline instead of opening a file. This is a code injection
# vector when the path comes from untrusted input.
#
# Expected: tclsh runs without error.

# Safe usage: opening a known file
set tmpfile [file join [file tempdir] w103_test.txt]
set fd [open $tmpfile w]
puts $fd "test data"
close $fd

# Read it back
set fd [open $tmpfile r]
set data [read $fd]
close $fd
puts "file content: [string trim $data]"

# Risky: a variable could start with "|"
set filename $tmpfile
# This is fine — it's a normal file path:
set fd [open $filename r]
close $fd
puts "open with variable: OK (was a normal path)"

# If filename were "|rm -rf /" this would execute a command!
# The LSP flags this because it cannot prove the variable is safe.

file delete $tmpfile
puts "W103 test complete"
