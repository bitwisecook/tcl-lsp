namespace eval N {proc tail {} {return OK}}
namespace eval { ]::inert} {proc tail {} {return NESTED}}
set prefix ::N
set tail tail
foreach invocation [list {[set prefix]::tail} {[list ::N]::tail} {[list " ]::inert"]::tail} {[format %s " ]::inert"]::tail} {[list ::N]} {[set prefix]::$tail} {[set prefix]::[set tail]}] {
    set code [catch {uplevel #0 $invocation} result]
    if {$code == 0} {puts [list $invocation $code $result]} else {puts [list $invocation $code ERROR]}
}
