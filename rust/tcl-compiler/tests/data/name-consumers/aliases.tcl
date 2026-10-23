namespace eval ns {}
interp alias {} ::pick {} list global
interp alias {} ::ns::pick {} list current
interp alias {} ::ns:: {} list empty
puts [namespace eval ns {pick}]
puts [::ns:::pick]
puts [::ns:::]
namespace eval other {proc pick {} {return ordinary}}
puts [namespace eval other {pick}]
if {[package vcompare [info tclversion] 8.5] >= 0} {
    namespace eval via {}
    namespace eval caller {namespace path ::via}
    interp alias {} ::via::pick {} list path
    puts [namespace eval caller {pick}]
    namespace eval via {proc local {} {return ordinary_path}}
    interp alias {} ::local {} list global_alias
    puts [namespace eval caller {local}]
}
