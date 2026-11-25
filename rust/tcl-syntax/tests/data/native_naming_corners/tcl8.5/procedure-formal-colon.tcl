set code [catch {proc p {:odd n::q} {list ${:odd} ${n::q} [info locals]}; p A B} result]
binary scan $result H* hex
puts [list $code $hex]
