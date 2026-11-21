oo::class create C {
    self method init {} { variable t classvar }
    method get {} { classvariable t; return $t ;# READ
    }
}
C init
puts [[C new] get]
