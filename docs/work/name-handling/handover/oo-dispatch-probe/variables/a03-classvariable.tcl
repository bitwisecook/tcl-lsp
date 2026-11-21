oo::class create C {
    self method setup {} { variable total; set total 7 }
    method get {} { classvariable total; return $total }
}
C setup
puts [[C new] get]
