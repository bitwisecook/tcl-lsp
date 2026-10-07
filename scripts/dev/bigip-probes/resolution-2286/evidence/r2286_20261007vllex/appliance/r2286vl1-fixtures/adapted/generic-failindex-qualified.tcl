proc hex {s} {binary scan $s H* out; return $out}
namespace eval N {}
set n {::N::v}
set c [catch {set h string; $h is alpha -failindex $n a2} r]
set exists [info exists $n]
set rc [catch {set $n} got]
set summary [list $c [hex $r] $exists $rc [hex $got]]
set summary
