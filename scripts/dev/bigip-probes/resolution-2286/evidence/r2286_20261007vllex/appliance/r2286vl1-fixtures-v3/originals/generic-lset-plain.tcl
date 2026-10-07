proc hex {s} {binary scan $s H* out; return $out}
namespace eval N {}
set n {v}
set c [catch {set $n {A B}; set h lset; $h $n 1 C} r]
set exists [info exists $n]
set rc [catch {set $n} got]
set summary [list $c [hex $r] $exists $rc [hex $got]]
puts $summary
set summary
