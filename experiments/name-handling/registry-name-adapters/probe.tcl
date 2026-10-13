foreach name {adapter::f ::adapter::f :::adapter::f ::::adapter::f adapter:::f adapter::::f :adapter::f adapter::f:} {
    namespace eval adapter {}
    proc adapter::f {} {return winner}
    set result [catch {eval [list $name]} value]
    puts [list lookup $name $result $value]
}
foreach name {tcl::dict::create ::tcl::dict::create :::tcl:::dict:::create ::::tcl::::dict::::create tcl::dictionary::create} {
    set result [catch {namespace which -command $name} value]
    puts [list private $name $result $value]
}
foreach name {if ::if :::if ::::if :if ::::if:} {
    set result [catch {eval [list $name 1 {set adapterVersionMarker reached}]} value]
    puts [list script $name $result $value]
}
