oo::class create C {
    constructor {} { my variable x; set x 5 }
    method get {} { my variable x; return $x }
}
puts [[C new] get]
