namespace eval nh {
    namespace ensemble create
    proc send {fixed value} {list old $fixed $value}
}
set before [nh send {a b} {c d}]
rename nh::send nh::saved
proc nh::send {fixed value} {list replacement $fixed $value}
list $before [nh send {a b} {c d}]
