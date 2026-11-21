oo::class create B {
    variable v
    constructor {a} { set v $a }
    method v {} { return $v }
}
oo::class create D {
    superclass B
    constructor {a b} { next $a }
}
puts [[D new 1 2] v]
