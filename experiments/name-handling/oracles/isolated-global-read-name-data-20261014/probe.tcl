set {::$literal} SCALAR
set {::arr($literal)} ELEMENT
set {::arr()} EMPTY_ELEMENT
set {::} EMPTY_SCALAR
proc read_names {} {list ${::$literal} ${::arr($literal)} ${::arr()} ${::}}
puts [list code [catch {read_names} result] result $result]
puts [list bare_exists [info exists ::literal]]
puts [list array_exists [info exists {::arr()}]]
