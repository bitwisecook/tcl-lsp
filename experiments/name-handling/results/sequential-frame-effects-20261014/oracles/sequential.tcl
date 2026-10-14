set x ROOT
namespace eval N {
 proc p {} {
  set x LOCAL
  set root [uplevel #0 {set x}]
  set current [uplevel 0 {set x}]
  return [list $root $current]
 }
}
puts [N::p]
puts [list [info commands ::uplevel] [info commands ::rename]]
