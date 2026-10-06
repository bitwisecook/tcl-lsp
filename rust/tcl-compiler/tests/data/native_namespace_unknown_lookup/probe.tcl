set ::called 0
proc ::fallback {args} {incr ::called; return FALLBACK}
namespace eval ::N {}
set supported [catch {namespace eval ::N {namespace unknown [list ::fallback]}} message]
if {$supported} {puts [list unsupported $supported $message]; exit}
namespace eval ::N {
    proc known {} {return KNOWN}
    proc existing {} {known}
    proc compiler_missing {} {return OK; unavailable_command}
    proc runtime_missing {} {unavailable_command}
    proc runtime_absolute_missing {} {::unavailable_command}
    proc existing_absolute {} {::N::known}
}
puts [list existing [::N::existing] $::called]
puts [list compiler [::N::compiler_missing] $::called]
puts [list fallback [::N::runtime_missing] $::called]
puts [list rootedExisting [::N::existing_absolute] $::called]
puts [list rootedFallback [::N::runtime_absolute_missing] $::called]
