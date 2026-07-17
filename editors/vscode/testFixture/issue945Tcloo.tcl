oo::class create Vault {
    method _secret {} { return hidden }
}
set v [Vault new]
$v _secret
oo::class create Animal {
    method speak {} { return animal }
}
oo::class create Dog {
    superclass Animal
    method speak {} { return dog }
}
set d [Dog new]
$d speak
oo::class create C {}
proc a {} {
    set o [C new]
    oo::objdefine $o {method m {} {return one}}
    $o m
}
proc b {} {
    set o [C new]
    oo::objdefine $o {method m {} {return two}}
    $o m
}
