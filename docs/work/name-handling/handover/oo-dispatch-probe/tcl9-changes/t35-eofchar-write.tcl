set f [open o.txt w]
puts [catch {fconfigure $f -eofchar x} r]:$r
close $f
