set f [open data.bin wb]
puts -nonewline $f "x\xFFy"
close $f
# 9.0: channels default to -profile strict on input
set f [open data.bin r]
fconfigure $f -encoding utf-8
puts "read: [catch {read $f} r] [string length $r]"
close $f
