interp alias {} ::makePrefix {} ::list
oo::class create C {
    method tick {fixed} {set ::observed [list tick $fixed]}
    method setup {destination} {
        set cb [namespace code [::makePrefix my tick {a b}]]
        set $destination {my unrelated}
        after idle $cb
    }
}
C create object; object setup cb; update; set ::observed
