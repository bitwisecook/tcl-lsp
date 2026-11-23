# Channels are per-interpreter: share makes one visible in both, transfer moves it.
set f [open data.txt w]
puts $f "line one"
close $f
set c [interp create]
set ch [open data.txt r]
puts "visible-before=[interp eval $c [list catch [list gets $ch] m]]"
interp share {} $ch $c
puts "shared-read=[interp eval $c [list gets $ch]]"
close $ch
puts "parent-closed-child-still-open=[interp eval $c [list catch [list eof $ch]]]"
interp eval $c [list close $ch]
set ch2 [open data.txt r]
interp transfer {} $ch2 $c
puts "parent-after-transfer=[catch {gets $ch2} m]"
puts "child-reads=[interp eval $c [list gets $ch2]]"
interp eval $c [list close $ch2]
interp delete $c
file delete data.txt
