proc hex {s} {binary scan $s H* out; return $out}
namespace eval N {}
set n {v}
set c [catch {set $n A; set h append; $h $n B C} r]
set exists [info exists $n]
set rc [catch {set $n} got]
set summary [list $c [hex $r] $exists $rc [hex $got]]
set summary
