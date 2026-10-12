namespace eval R {set v 1}
proc a {} {global ::R:::v; return [info vars]}
proc b {} {variable ::R:::v; return [info vars]}
proc c {} {global R:::v; return [info vars]}
proc d {} {variable R:::v; return [info vars]}
foreach p {a b c d} {puts "$p:[catch {$p} value]:$value"}
