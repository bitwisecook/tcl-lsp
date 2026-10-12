interp alias {} ::makePrefix {} ::list
oo::class create C {
    method tick {fixed} {set ::observed [list tick $fixed]}
    method setup {} {
        set cb [namespace code [::makePrefix my tick {a b}]]
        after idle $cb
    }
}
C create object; object setup; update; set ::observed
