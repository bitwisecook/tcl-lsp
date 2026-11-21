oo::class create C {
    variable a b
    method get {} { set a 1; set b 2; return [list $a $b] }
}
oo::define C variable -append c
oo::define C method more {} { set c 3; return $c }
puts [[C new] get][[C new] more]
