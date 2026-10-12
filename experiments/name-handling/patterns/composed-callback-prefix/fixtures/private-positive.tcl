interp alias {} ::makePrefix {} ::list
oo::class create C {
    method tick {fixed} {}
    unexport tick
    method setup {} {
        set cb [namespace code [::makePrefix my tick {a b}]]
        after idle $cb
    }
}
