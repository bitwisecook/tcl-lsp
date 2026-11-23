# chan close with a direction (8.6+): file, bidirectional pipe
proc t {label script} {if {[catch {uplevel 1 $script} r]} {puts "$label: ERR $r"} else {puts "$label: $r"}}
t file-read {set f [open c05.txt w]; set r [catch {chan close $f read} m]; catch {close $f}; list $r $m}
t file-write {set f [open c05.txt w]; set r [catch {chan close $f write} m]; set ex [expr {$f in [file channels]}]; list $r $m $ex}
t pipe-half {set p [open |cat r+]; puts $p hello; chan close $p write; set l [gets $p]; close $p; set l}
t bad-dir {set f [open c05.txt w]; catch {close $f sideways} m; catch {close $f}; set m}
t arity {catch {close} m; set m}
