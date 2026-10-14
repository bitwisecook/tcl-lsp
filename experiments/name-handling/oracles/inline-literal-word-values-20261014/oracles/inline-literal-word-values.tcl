proc show {value} {return $value}
puts [list bare [show a\ b]]
puts [list quoted [show "a\x20b"]]
puts [list braced [show {a b}]]
puts [list literal [show {$literal [missing]}]]
puts [list empty [show {}]]
