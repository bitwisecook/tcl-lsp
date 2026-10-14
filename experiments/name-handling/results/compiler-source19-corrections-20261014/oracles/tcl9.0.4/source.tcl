namespace eval ::inner {proc p {} {return GLOBAL}}
namespace eval ::ns::inner {proc p {} {return LOCAL}}
proc ::ns::doit {} {rename inner::p inner::saved}
::ns::doit
puts [list [::inner::p] [::ns::inner::saved] [llength [info commands ::ns::inner::p]]]
