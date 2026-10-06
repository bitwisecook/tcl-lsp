puts [list sqrt(2) [expr {sqrt(2)}]]
puts [list sin(1) [expr {sin(1)}]]
puts [list log(2) [expr {log(2)}]]
puts [list floor(-1.2) [expr {floor(-1.2)}]]
set code [catch {expr {srand(1)}} value]
puts [list seed 1 $code $value]
if {!$code} {puts [list draws [expr {rand()}] [expr {rand()}] [expr {rand()}]]}
set code [catch {expr {srand(-1)}} value]
puts [list seed -1 $code $value]
if {!$code} {puts [list draws [expr {rand()}] [expr {rand()}] [expr {rand()}]]}
set code [catch {expr {srand(0)}} value]
puts [list seed 0 $code $value]
if {!$code} {puts [list draws [expr {rand()}] [expr {rand()}] [expr {rand()}]]}
set code [catch {expr {srand(4294967297)}} value]
puts [list seed 4294967297 $code $value]
if {!$code} {puts [list draws [expr {rand()}] [expr {rand()}] [expr {rand()}]]}
set code [catch {expr {srand(9223372036854775807)}} value]
puts [list seed 9223372036854775807 $code $value]
if {!$code} {puts [list draws [expr {rand()}] [expr {rand()}] [expr {rand()}]]}
set code [catch {expr {srand(9223372036854775808)}} value]
puts [list seed 9223372036854775808 $code $value]
if {!$code} {puts [list draws [expr {rand()}] [expr {rand()}] [expr {rand()}]]}
set code [catch {expr {srand(-9223372036854775809)}} value]
puts [list seed -9223372036854775809 $code $value]
if {!$code} {puts [list draws [expr {rand()}] [expr {rand()}] [expr {rand()}]]}
set code [catch {expr {srand(1.5)}} value]
puts [list seed 1.5 $code $value]
if {!$code} {puts [list draws [expr {rand()}] [expr {rand()}] [expr {rand()}]]}
set tcl_precision 12; set saved [expr {sqrt(2)}]; set tcl_precision 4; puts [list precision-first $saved]; set tcl_precision 12; puts [list precision-cached $saved]; puts [list precision-fresh [expr {sqrt(2)}]]
