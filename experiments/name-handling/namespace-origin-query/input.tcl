puts [list IDENTITY [info patchlevel]]
proc ::__no_observe {label script} {
    unset -nocomplain ::errorCode ::errorInfo
    set code [catch {uplevel 1 $script} value]
    set errorcode {}
    if {$code && [info exists ::errorCode]} {set errorcode $::errorCode}
    puts [list OBSERVATION $label $code $value $errorcode]
}
proc ::__no_target {} {return ROOT}
proc ::__no_other {} {return OTHER}
proc :__no_literal {} {return COLON}
proc ::__no_tail: {} {return TAIL}
proc [list __no_missing {a b}] {} {return PREFIXNAME}
set ::__no_jim_alias [expr {[llength [info commands alias]] > 0}]
proc ::__no_make_alias {name args} {
    if {$::__no_jim_alias} {
        eval [linsert $args 0 alias $name]
    } else {
        eval [linsert $args 0 interp alias {} $name {}]
    }
    return {}
}
foreach {label source} {
    root-bare {namespace origin __no_target}
    root-absolute {namespace origin ::__no_target}
    root-extra-colons {namespace origin ::::__no_target}
    root-single-colon {namespace origin :__no_literal}
    root-single-colon-display {namespace origin :::__no_literal}
    root-tail-colon {namespace origin ::__no_tail:}
    root-missing {namespace origin __no_absent}
} {::__no_observe $label $source}
::__no_observe alias-setup {::__no_make_alias ::__no_alias ::__no_target}
::__no_observe alias-query {namespace origin __no_alias}
::__no_observe alias-absolute-query {namespace origin ::__no_alias}
::__no_observe alias-prefix-setup {::__no_make_alias ::__no_prefix __no_missing {a b}}
::__no_observe alias-prefix-query {namespace origin __no_prefix}
::__no_observe alias-missing-setup {::__no_make_alias ::__no_missing_alias __no_absent ARG}
::__no_observe alias-missing-query {namespace origin __no_missing_alias}
::__no_observe alias-chain-setup {::__no_make_alias ::__no_chain ::__no_alias}
::__no_observe alias-chain-query {namespace origin __no_chain}
set ::__no_held ::__no_alias
::__no_observe retained-alias-before {namespace origin $::__no_held}
::__no_observe alias-rename-setup {rename ::__no_alias ::__no_old_alias}
::__no_observe alias-replacement-setup {::__no_make_alias ::__no_alias ::__no_other}
::__no_observe retained-alias-after {namespace origin $::__no_held}
::__no_observe renamed-alias-query {namespace origin ::__no_old_alias}
::__no_observe chain-after-replacement {namespace origin __no_chain}
namespace eval ::__no_context {
    proc __no_target {} {return LOCAL}
    ::__no_observe local-shadow {namespace origin __no_target}
    ::__no_observe absolute-root {namespace origin ::__no_target}
    ::__no_observe root-fallback {namespace origin __no_other}
    ::__no_observe local-alias-setup {::__no_make_alias __no_local_alias __no_target}
    ::__no_observe local-alias-query {namespace origin __no_local_alias}
    ::__no_observe root-alias-from-child {namespace origin ::__no_alias}
}
namespace eval ::__no_source {
    proc exported {} {return EXPORTED}
    ::__no_observe export-setup {namespace export exported}
}
namespace eval ::__no_destination {
    ::__no_observe import-setup {namespace import ::__no_source::exported}
    ::__no_observe import-origin {namespace origin exported}
}
namespace eval ::__no_a: {
    namespace eval b {
        proc marker {} {return LEFT}
        ::__no_observe left-current {namespace current}
        ::__no_observe left-origin {namespace origin marker}
        ::__no_observe left-report-requery {namespace origin [namespace origin marker]}
    }
}
namespace eval ::__no_a {
    namespace eval :b {
        proc marker {} {return RIGHT}
        ::__no_observe right-current {namespace current}
        ::__no_observe right-origin {namespace origin marker}
        ::__no_observe right-report-requery {namespace origin [namespace origin marker]}
    }
}
