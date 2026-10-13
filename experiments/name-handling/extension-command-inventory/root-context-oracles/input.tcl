proc capture {label script} {
    set code [catch {uplevel #0 $script} result]
    puts [list ROW $label $code $result]
}
proc lost {} {return PLAIN}
proc :lost {} {return COLON}
namespace eval :N {proc x {} {return LEADING_NAMESPACE}}
namespace eval N {proc x {} {return ORDINARY_NAMESPACE}}
namespace eval N: {proc x {} {return TRAILING_NAMESPACE}}
namespace eval {N[0]} {proc {p*} {} {return BRACKET_NAMESPACE}}
namespace eval N0 {proc pZ {} {return SIBLING_NAMESPACE}}
capture root-original {:lost}
capture root-other {lost}
capture root-colon-query {namespace which -command :lost}
capture root-plain-query {namespace which -command lost}
capture leading-namespace-original {:N::x}
capture leading-namespace-rooted-report {:::N::x}
capture leading-namespace-inscope {namespace inscope :N x}
capture trailing-namespace-root-join {N:::x}
capture trailing-namespace-inscope {namespace inscope N: x}
capture bracket-root-context [list {N[0]::p*}]
capture bracket-rooted-context [list {::N[0]::p*}]
capture live-root-commands {info commands}
