proc hex {s} {binary scan $s H* out; return $out}
namespace eval N {}
set n {arr(k::part)}
set c [catch {set h lassign; $h {A B} $n} r]
set exists [info exists $n]
set rc [catch {set $n} got]
set summary [list $c [hex $r] $exists $rc [hex $got]]
puts $summary
set summary
