set bodies {
  {switch -- $k a {my ping $k}}
  {switch -- $k {a {my ping $k}}}
  {if {1} {my ping $k}}
  {if {0} {} elseif {1} {my ping $k}}
  {if {0} {} else {my ping $k}}
  {set i 0; while {$i < 1} {incr i; my ping $k}}
  {for {set i 0} {$i < 1} {incr i} {my ping $k}}
  {foreach i {1} {my ping $k}}
  {catch {my ping $k}}
  {try {my ping $k} finally {}}
  {eval {my ping $k}}
  {switch -glob -- $k {a* {if {1} {catch {my ping $k}}}}}
}
set n 0
foreach b $bodies {
  incr n
  oo::class create C$n "method ping {k} {set ::hit 1}; [list method get {k} $b]"
  set ::hit 0; [C$n new] get a
  puts "$n $::hit"
}
