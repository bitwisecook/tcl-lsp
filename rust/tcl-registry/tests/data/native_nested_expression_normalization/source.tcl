proc original {x} {expr {[expr {$x+1}]}}
proc rewritten {x} {expr {$x+1}}
foreach value {2 abc 1.0 Inf NaN} {
 foreach target {original rewritten} {
  set code [catch [list $target $value] message]
  set metadata {}
  if {$code == 1 && [info exists ::errorCode]} {lappend metadata errorCode $::errorCode}
  if {$code == 1 && [info exists ::errorInfo]} {lappend metadata errorInfo $::errorInfo}
  puts [list ROW $target $value $code $message $metadata]
 }
}
