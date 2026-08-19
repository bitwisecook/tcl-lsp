proc boom {n1 n2 op} { error "kaboom" }
set v old
trace add variable v write boom
puts "set-existing: [catch {set v new} e] / $e / value=[set v]"

trace add variable w write boom
puts "set-fresh: [catch {set w new} e] / $e / exists=[info exists w] / value=[expr {[info exists w] ? [set w] : {<none>}}]"

array set a {k old}
trace add variable a(k) write boom
puts "elem-existing: [catch {set a(k) new} e] / $e / value=$a(k)"

array set b {}
trace add variable b(j) write boom
puts "elem-fresh: [catch {set b(j) new} e] / $e / exists=[info exists b(j)] / value=[expr {[info exists b(j)] ? $b(j) : {<none>}}]"

set ap abc
trace add variable ap write boom
puts "append: [catch {append ap def} e] / $e / value=[set ap]"

set ic 5
trace add variable ic write boom
puts "incr: [catch {incr ic} e] / $e / value=[set ic]"

set lp {a b}
trace add variable lp write boom
puts "lappend: [catch {lappend lp c} e] / $e / value=[set lp]"

set ap2 ""
trace add variable ap2 write boom
puts "append-fresh-empty: [catch {append ap2 zz} e] / value=[set ap2]"

trace add variable nl write boom
puts "lappend-fresh: [catch {lappend nl x} e] / exists=[info exists nl] / value=[expr {[info exists nl] ? [set nl] : {<none>}}]"

trace add variable ni write boom
puts "incr-fresh: [catch {incr ni} e] / $e / exists=[info exists ni] / value=[expr {[info exists ni] ? [set ni] : {<none>}}]"

trace add variable na write boom
puts "append-fresh: [catch {append na q} e] / exists=[info exists na] / value=[expr {[info exists na] ? [set na] : {<none>}}]"
