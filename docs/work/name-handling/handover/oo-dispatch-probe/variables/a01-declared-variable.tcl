oo::class create C {
    variable count
    constructor {} { set count 0 }
    method bump {} { incr count; return $count }
}
puts [[C new] bump]
