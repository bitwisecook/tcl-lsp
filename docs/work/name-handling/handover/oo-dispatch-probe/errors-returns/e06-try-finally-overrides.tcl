# Errors in handlers and in finally; the -during key.
proc p1 {} { try { error first } on error {m} { error second } }
proc p2 {} { try { return body } finally { error infinally } }
proc p3 {} { try { return body } finally { return fromfinally } }
proc p4 {} { try { error a } finally { set ::seen 1 } }
proc p5 {} { try { error a } finally { error b } }
foreach p {p1 p2 p3 p4 p5} {
  set c [catch {$p} m o]
  if {[dict exists $o -during]} { set dd [dict get $o -during]; set d "[dict get $dd -code]/[expr {[dict exists $dd -errorcode] ? [dict get $dd -errorcode] : {-}}]" } else { set d - }
  puts "$p: $c [list $m] during=$d"
}
puts "seen: $::seen"
