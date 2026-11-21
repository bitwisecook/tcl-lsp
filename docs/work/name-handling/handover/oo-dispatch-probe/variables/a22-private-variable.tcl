
oo::class create C {
    private variable secret
    constructor {} { set secret s }
    method get {} { return $secret }
}
puts [[C new] get]
