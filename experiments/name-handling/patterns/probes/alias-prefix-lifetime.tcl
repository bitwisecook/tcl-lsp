namespace eval ::nh {proc emit {fixed value} {list $fixed $value}}
interp alias {} ::nh::local {} ::nh::emit {a b}
rename ::nh::local ::nh::renamed
set description [interp alias {} ::nh::local]
list $description [::nh::renamed {c d}]

