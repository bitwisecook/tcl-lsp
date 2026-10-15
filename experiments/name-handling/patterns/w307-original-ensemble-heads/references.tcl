namespace eval N {proc tail {} {return OK}}
set prefix ::N
set { with space } ::N
set {$literal} ::N
set {literal(open} ::N
set {} ::N
set {a{b}c} ::N
set café ::N
foreach invocation [list {${prefix}::tail} {${ with space }::tail} {${$literal}::tail} {${literal(open}::tail} {${}::tail} {${a{b}c}::tail} {$café::tail} {$prefix::tail}] {
    set code [catch {uplevel #0 $invocation} result]
    if {$code == 0} {puts [list $invocation $code $result]} else {puts [list $invocation $code ERROR]}
}
