# The optimiser's output (O109 removed the auto_path write).
file mkdir sv19-pkg
set f [open sv19-pkg/pkgIndex.tcl w]
puts $f {package ifneeded svprobe 1.0 {package provide svprobe 1.0}}
close $f

puts [catch {package require svprobe} v]:$v
