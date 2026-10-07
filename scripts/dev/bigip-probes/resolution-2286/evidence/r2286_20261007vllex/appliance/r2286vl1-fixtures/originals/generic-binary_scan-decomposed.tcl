proc hex {s} {binary scan $s H* out; return $out}
namespace eval N {}
set n "e\u0301"
set c [catch {set h binary; $h scan [binary format H* 4142] a* $n} r]
set exists [info exists $n]
set rc [catch {set $n} got]
set summary [list $c [hex $r] $exists $rc [hex $got]]
puts $summary
set summary
