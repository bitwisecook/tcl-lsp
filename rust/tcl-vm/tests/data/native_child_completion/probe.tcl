set cases {
    {set x [binary format H* 41];set x}
    {catch {binary format H* 41}}
    {subst {[binary format H* 41]}}
    {expr {[string length [binary format H* 41]] + 4}}
    {if {[string length [binary format H* 41]]} {set x ok} else {set x bad};set x}
    {set out {};foreach x {41 42} {lappend out [binary format H* $x]};set out}
    {catch {binary format INVALID}}
    {catch {binary scan [binary format H* 41] H* h};set h}
}
set index 0
foreach source $cases {
    set code [catch $source result]
    binary scan $source H* source_hex
    binary scan $result H* result_hex
    puts "$index\t$source_hex\t$code\t$result_hex"
    incr index
}
