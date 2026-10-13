puts [list version [info patchlevel]]
if {[llength [info commands dict]] == 0} {
 puts [list dict unsupported]
} else {
 set code [catch {interp create named} child]
 puts [list named-child $code $child]
 if {$code == 0} {
  set d {child other}
  dict with d {}
  puts [list overwritten-handle $child]
  puts [list eval-after-write [catch {interp eval $child {set x VALUE}} result] $result]
  interp delete named
 }
}
