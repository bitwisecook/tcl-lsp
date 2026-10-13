proc capture {label script} {
    set code [catch {uplevel #0 $script} result]
    puts [list ROW $label $code $result]
}
proc escape {value} {return [string map [list "\\" "\\\\" "*" "\\*" "?" "\\?" "\[" "\\\[" "\]" "\\\]"] $value]}
capture patchlevel {::info patchlevel}
capture version {::info version}
capture jim-version {::set ::jim_version}
capture tcl-version {::set ::tcl_version}

foreach ns {::N[0] ::N0 ::Nmissing ::W* ::WZ ::Q? ::Qa {::B\slash}} {::namespace eval $ns {}}
foreach name {::N[0]::p ::N0::p ::Nmissing::absent ::W*::p ::WZ::p ::Q?::p ::Qa::p {::B\slash::p} ::N[0]::p* ::N[0]::pZ ::N[0]::p? ::N[0]::pA ::N[0]::p[0] ::N[0]::p0 {::N[0]::p\tail} ::N[0]:: :p p* pZ p? pA p[0] p0 {p\tail}} {
    ::proc $name {} [list ::return EXACT]
}
foreach pair {
    {::N[0]::p {::N[0]::p} 1}
    {::W*::p {::W*::p} 1}
    {::Q?::p {::Q?::p} 1}
    {{::B\slash::p} {::B\slash::p} 1}
    {::N[0]::p* {::N[0]::p\*} 1}
    {::N[0]::p? {::N[0]::p\?} 1}
    {::N[0]::p[0] {::N[0]::p\[0\]} 1}
    {{::N[0]::p\tail} {::N[0]::p\\tail} 1}
    {::N[0]:: {::N[0]::} 1}
    {::N[missing]::absent {::N[missing]::absent} 0}
    {::N[0]::absent* {::N[0]::absent\*} 0}
    {:p :p 1}
    {p* {p\*} 1}
    {absent* {absent\*} 0}
    {::N[0]::p:::missing {::N[0]::p:::missing} 0}
} {
    foreach {name cpattern expected} $pair break
    capture [list cpattern $name $expected] [list ::info commands $cpattern]
    capture [list jpattern $name $expected] [list ::info commands [escape $name]]
    capture [list original-call $name $expected] [list $name]
}
