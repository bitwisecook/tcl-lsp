set x ROOT
proc outer {} {uplevel #0 {
 proc inner {} {
  set x INNER
  puts $x
 }
}}
outer
inner
