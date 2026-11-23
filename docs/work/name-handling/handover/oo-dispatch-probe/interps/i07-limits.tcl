# interp recursionlimit and interp limit (commands and time limits, 8.5+)
set c [interp create]
puts "default=[interp recursionlimit $c]"
interp recursionlimit $c 50
puts "set=[interp recursionlimit $c]"
interp eval $c { proc r {n} { r [incr n] } }
puts "deep=[catch {interp eval $c {r 0}} m] [string range $m 0 40]"
puts "child-view=[interp eval $c {interp recursionlimit {}}]"
interp limit $c commands -value 100
puts "limit=[interp limit $c commands -value]"
puts "exceeded=[catch {interp eval $c {while 1 {incr i}}} m] $m"
interp delete $c
