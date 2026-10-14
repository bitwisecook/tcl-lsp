proc hex {s} {binary scan $s H* result; return $result}
interp create kid
foreach source {::missing N::held plain} {
    set code [catch {interp expose kid $source allowed} result]
    puts [list [hex $source] $code [hex $result]]
}
interp delete kid
