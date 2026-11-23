# Jim GC: a finalizer runs when collect frees an unreferenced reference
proc fin {r v} {puts "finalized $v"}
set r [ref val tag fin]
puts [string length [finalize $r]]
unset r
puts "collected [collect]"
