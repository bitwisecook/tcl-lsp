namespace eval nh {proc emit {fixed value} {list old $fixed $value}}
alias ::nh::local ::nh::emit {a b}
rename ::nh::local ::nh::renamed
set description [info alias ::nh::renamed]
rename ::nh::emit ::nh::saved
proc ::nh::emit {fixed value} {list replacement $fixed $value}
list $description [::nh::renamed {c d}]
