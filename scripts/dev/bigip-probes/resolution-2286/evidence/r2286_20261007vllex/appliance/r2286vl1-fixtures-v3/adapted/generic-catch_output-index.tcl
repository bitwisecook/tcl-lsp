proc hex {s} {binary scan $s H* out; return $out}
namespace eval N {}
set n {arr(k::part)}
set c [catch {set h catch; $h {set result RESULT} $n} r]
set exists [info exists $n]
set rc [catch {set $n} got]
set summary [list $c [hex $r] $exists $rc [hex $got]]
set summary
