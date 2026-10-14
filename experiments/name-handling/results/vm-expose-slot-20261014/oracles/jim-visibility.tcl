proc hex {s} {binary scan $s H* result; return $result}
set kid [interp]
$kid eval {proc kept {} {return kept}}
foreach op {hide expose} {
    set code [catch {$kid $op kept hidden} result]
    puts [list $op $code [hex $result]]
}
$kid delete
