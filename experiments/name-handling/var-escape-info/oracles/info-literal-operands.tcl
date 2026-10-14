foreach member {cancel constants consts} {
 set code [catch [list info $member] result]
 puts [list member $member $code $result]
}
set {$literal} 1
set {[literal]} 1
puts [list braced_dollar [info exists {$literal}]]
puts [list escaped_dollar [info exists \$literal]]
puts [list braced_brackets [info exists {[literal]}]]
puts [list escaped_brackets [info exists \[literal\]]]
