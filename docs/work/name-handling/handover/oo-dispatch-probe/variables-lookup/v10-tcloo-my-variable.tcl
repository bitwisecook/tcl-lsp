oo::class create C {
    constructor {} { my variable x; set x myvar }
    method get {} { my variable x; return $x ;# READ
    }
}
puts [[C new] get]
