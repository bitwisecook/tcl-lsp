namespace eval ::nh {proc emit {} {return first}}
set ns ::nh
set head ${ns}::emit
rename ::nh::emit ::nh::old
proc ::nh::emit {} {return replacement}
$head

