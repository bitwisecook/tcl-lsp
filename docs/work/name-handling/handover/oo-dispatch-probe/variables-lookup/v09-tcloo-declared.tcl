oo::class create C {
    variable count
    constructor {} { set count declared }
    method get {} { return $count ;# READ
    }
}
puts [[C new] get]
