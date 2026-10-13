proc capture {label script} {set code [catch {uplevel #0 $script} result]; puts [list ROW $label $code $result]}
proc :lost {} {return COLON}
proc lost {} {return PLAIN}
namespace eval {N[0]} {proc {p*} {} {return BRACKET}}
namespace eval N0 {proc pZ {} {return SIBLING}}
foreach query {{::info commands} {::info -nons commands} {::info version}} {capture $query $query}
foreach word {:lost lost {N[0]::p*} {::N[0]::p*} N0::pZ} {
    capture [list exists $word] [list ::exists -command $word]
    capture [list call $word] [list $word]
}
set bad {}
set count 0
if {![catch {::info -nons commands} names]} {
    foreach name $names {
        if {![::exists -command $name]} {lappend bad $name}
        incr count
    }
}
puts [list RAW-ROOT-KEYS $count UNCALLABLE $bad]
