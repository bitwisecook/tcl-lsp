namespace eval ::nh {
    proc emit {fixed value} {list $fixed $value}
    namespace ensemble create -command facade -map [list send [list ::nh::emit {a b}]]
}
::nh::facade send {c d}

