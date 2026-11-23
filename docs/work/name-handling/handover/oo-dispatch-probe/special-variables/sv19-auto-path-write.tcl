# A write to auto_path that no script reads still changes what package require finds.
# (Writes a one-line package into ./sv19-pkg of the current directory first.)
file mkdir sv19-pkg
set f [open sv19-pkg/pkgIndex.tcl w]
puts $f {package ifneeded svprobe 1.0 {package provide svprobe 1.0}}
close $f
set auto_path [linsert $auto_path 0 sv19-pkg]
puts [catch {package require svprobe} v]:$v
