namespace eval n {}
foreach script {{namespace upvar n} {namespace upvar n a} {namespace upvar n a b}} {set c [catch $script m]; puts [list $script $c $m]}
