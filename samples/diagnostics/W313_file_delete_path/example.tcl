# W313: file delete with a variable path — path-traversal risk
# Source: SpiceGenTcl/src/ltspice/specSimulatorClassesLtspice.tcl:93-97
#
# Deleting files using a variable path without validation could allow
# path traversal (e.g. "../../etc/passwd"). The fix is to normalize
# and validate the path stays within the intended directory.
#
# Expected: tclsh runs without error.

# Create a temp file to delete
set tmpdir [file tempdir]
set testfile [file join $tmpdir w313_test.txt]
close [open $testfile w]

# Risky: variable path without validation (W313)
set path $testfile
file delete $path
puts "deleted via variable path: $path"

# Safe: normalize and validate
set basedir $tmpdir
set userpath "w313_safe.txt"
set fullpath [file normalize [file join $basedir $userpath]]

# Verify the resolved path is still under basedir
if {[string equal -length [string length $basedir] $basedir $fullpath]} {
    close [open $fullpath w]
    file delete $fullpath
    puts "safe delete: validated path is under basedir"
} else {
    puts "BLOCKED: path traversal detected"
}

puts "W313 test complete"
