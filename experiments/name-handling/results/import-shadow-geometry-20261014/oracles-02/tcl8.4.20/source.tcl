namespace eval ::ns {proc {} {} {} }
proc ::ns {} {return ROOT}
puts [list [namespace which -command ::ns::] [namespace which -command ::ns]]
puts [list [namespace qualifiers ::ns::] [namespace qualifiers ::ns]]
