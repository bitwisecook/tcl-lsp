proc hex {s} {binary scan $s H* out; return $out}
namespace eval N {}
set n "e\u0301"
set c [catch {proc sample {{x DEFAULT}} {}; set h info; $h default sample x $n} r]
set exists [info exists $n]
set rc [catch {set $n} got]
set summary [list $c [hex $r] $exists $rc [hex $got]]
set summary
