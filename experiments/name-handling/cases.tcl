# Run in a fresh disposable interpreter; observations are not assertions.

proc nh_hex {value} {
    binary scan $value H* encoded
    return $encoded
}

proc nh_observe {id body} {
    set ::errorCode {}
    set ::errorInfo {}
    set code [catch {uplevel #0 $body} result]
    set errorcode {}
    set errorinfo {}
    if {$code == 1} {
        set errorcode $::errorCode
        set errorinfo $::errorInfo
    }
    puts [list OBS $id code $code result_hex [nh_hex $result] \
        errorcode_hex [nh_hex $errorcode] errorinfo_hex [nh_hex $errorinfo]]
}

puts [list META patchlevel [info patchlevel] tclversion [info tclversion]]

nh_observe NH01 {
    proc ::__nh_01_p {} {return GLOBAL}
    namespace eval ::__nh_01 {
        proc __nh_01_p {} {return CURRENT}
        __nh_01_p
    }
}

nh_observe NH02 {
    namespace eval ::__nh_02_target {proc p {} {return GLOBAL_TARGET}}
    namespace eval ::__nh_02 {
        namespace eval __nh_02_target {}
        __nh_02_target::p
    }
}

nh_observe NH03 {
    namespace eval ::__nh_03_path::inner {proc p {} {return PATH_TARGET}}
    namespace eval ::__nh_03 {
        namespace path ::__nh_03_path
        inner::p
    }
}

nh_observe NH04 {
    namespace eval ::__nh_04 {
        proc parent_only {} {return ANCESTOR}
        namespace eval child {parent_only}
    }
}

nh_observe NH05 {
    namespace eval ::__nh_05::n {variable x 0}
    namespace eval ::__nh_05 {
        proc run {} {set n::x 7; return $n::x}
        list [run] [set n::x]
    }
}

nh_observe NH06 {
    set ::__nh_06_x GLOBAL_VALUE
    namespace eval ::__nh_06 {set __nh_06_x}
}

nh_observe NH07 {
    set __nh_07_x 1
    set ::__nh_07_x 2
    list [set __nh_07_x] [set ::__nh_07_x]
}

nh_observe NH08 {
    set {} EMPTY
    set {$n} LITERAL_DOLLAR
    set {{a}} LITERAL_BRACES
    list [set {}] [set {$n}] [set {{a}}] [info exists {{a}}]
}

nh_observe NH09 {
    namespace eval ::__nh_09 {
        proc p {} {return NORMAL}
        proc {} {} {return EMPTY_TAIL}
    }
    list [::__nh_09:::p] [::__nh_09:::]
}

nh_observe NH10 {
    namespace eval ::__nh_10 {
        namespace eval a: {proc p {} {return FIRST_SLOT}}
        namespace eval a {proc :p {} {return SECOND_SLOT}}
        list [namespace eval a: {p}] [namespace eval a {:p}] \
            [namespace eval a: {namespace which p}] \
            [namespace eval a {namespace which :p}]
    }
}

nh_observe NH11 {
    namespace eval ::__nh_11_src {proc p {} {return ORIGINAL}; namespace export p}
    namespace eval ::__nh_11_dst {namespace import ::__nh_11_src::p}
    rename ::__nh_11_src::p ::__nh_11_src::pp
    namespace eval ::__nh_11_dst {list [p] [namespace origin p]}
}

nh_observe NH12 {
    namespace eval ::__nh_12_src {proc p {} {return OLD}; namespace export p}
    namespace eval ::__nh_12_dst {namespace import ::__nh_12_src::p}
    proc ::__nh_12_src::p {} {return NEW}
    namespace eval ::__nh_12_dst {p}
}

nh_observe NH13 {
    proc ::__nh_13_target {} {return OLD}
    interp alias {} ::__nh_13_alias {} ::__nh_13_target
    set before [::__nh_13_alias]
    proc ::__nh_13_target {} {return NEW}
    list $before [::__nh_13_alias]
}

nh_observe NH14 {
    namespace eval ::__nh_14 {
        proc run {} {set x 1; upvar 0 x y; set y 2; return $x}
        run
    }
}

nh_observe NH15 {
    namespace eval ::__nh_15 {
        variable x NAMESPACE_VALUE
        proc run {} {upvar 1 x y; return $y}
        run
    }
}

nh_observe NH16 {
    proc ::__nh_16_source {} {return GLOBAL_IMPORT}
    namespace eval :: {namespace export __nh_16_source}
    namespace eval ::__nh_16 {
        namespace import ::__nh_16_source
        set before [__nh_16_source]
        namespace forget ::__nh_16_source
        list $before [namespace which __nh_16_source] \
            [info commands ::__nh_16::__nh_16_source]
    }
}
