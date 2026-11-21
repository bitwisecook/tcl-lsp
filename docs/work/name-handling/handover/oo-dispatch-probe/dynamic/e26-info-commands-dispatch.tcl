namespace eval ::h { proc a {} { return A }; proc b {} { return B } }
foreach c [lsort [info commands ::h::*]] { puts -nonewline [$c] }
puts ""
