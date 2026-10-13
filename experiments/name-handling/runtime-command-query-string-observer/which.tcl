puts [list IDENTITY [info patchlevel]]
proc ::__nh_observe {label script} {
    unset -nocomplain ::errorCode ::errorInfo
    set code [catch {uplevel 1 $script} value]
    set errorcode {}
    switch -exact $code 0 {} default {switch -exact [info exists ::errorCode] 1 {set errorcode $::errorCode}}
    puts [list OBSERVATION $label $code $value $errorcode]
}
proc ::__nh_shared {} {return ROOT}
proc ::__nh_tail: {} {return TAIL}
proc :__nh_literal {} {return COLON}
foreach {label source} {
    root-bare {namespace which __nh_shared}
    root-command {namespace which -command __nh_shared}
    root-command-short {namespace which -c __nh_shared}
    root-command-prefix {namespace which -comman __nh_shared}
    root-absolute {namespace which -command ::__nh_shared}
    root-extra-colons {namespace which -command ::::__nh_shared}
    root-missing {namespace which -command __nh_absent}
    root-tail-colon {namespace which -command ::__nh_tail:}
    root-single-colon {namespace which -command :__nh_literal}
    root-single-colon-display {namespace which -command :::__nh_literal}
} {::__nh_observe $label $source}
::__nh_observe alias-setup {interp alias {} ::__nh_alias {} ::__nh_shared}
::__nh_observe alias-query {namespace which -command __nh_alias}
namespace eval ::__nh_p1 {proc __nh_shared {} {return PATH1};proc __nh_path_only {} {return PATH1}}
namespace eval ::__nh_p2 {proc __nh_path_only {} {return PATH2}}
namespace eval ::__nh_context {
    proc __nh_shared {} {return LOCAL}
    ::__nh_observe current-shadow {namespace which -command __nh_shared}
    ::__nh_observe current-absolute-root {namespace which -command ::__nh_shared}
    ::__nh_observe current-alias-root {namespace which -command __nh_alias}
    ::__nh_observe path-setup {namespace path {::__nh_p1 ::__nh_p2}}
    ::__nh_observe path-current-shadow {namespace which -command __nh_shared}
    ::__nh_observe path-first {namespace which -command __nh_path_only}
    rename __nh_shared {}
    ::__nh_observe path-over-root {namespace which -command __nh_shared}
    ::__nh_observe path-reverse-setup {namespace path {::__nh_p2 ::__nh_p1}}
    ::__nh_observe path-reverse {namespace which -command __nh_path_only}
    ::__nh_observe path-clear-setup {namespace path {}}
    ::__nh_observe cleared-root-fallback {namespace which -command __nh_shared}
    ::__nh_observe current-missing {namespace which -command __nh_absent}
}
namespace eval ::__nh_source {
    proc exported {} {return EXPORTED}
    ::__nh_observe export-setup {namespace export exported}
}
namespace eval ::__nh_target {
    ::__nh_observe import-setup {namespace import ::__nh_source::exported}
    ::__nh_observe import-query {namespace which -command exported}
}
namespace eval ::__nh_a: {
    namespace eval b {
        proc marker {} {return LEFT}
        ::__nh_observe colon-left-current {namespace current}
        ::__nh_observe colon-left-query {namespace which -command marker}
        ::__nh_observe colon-left-display-query {namespace which -command [namespace which -command marker]}
    }
}
namespace eval ::__nh_a {
    namespace eval :b {
        proc marker {} {return RIGHT}
        ::__nh_observe colon-right-current {namespace current}
        ::__nh_observe colon-right-query {namespace which -command marker}
        ::__nh_observe colon-right-display-query {namespace which -command [namespace which -command marker]}
    }
}
