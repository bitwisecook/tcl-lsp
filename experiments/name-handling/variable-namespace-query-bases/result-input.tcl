set ::__query_rows {}
lappend ::__query_rows [list PATCHLEVEL [info patchlevel]]
set ::g GLOBAL
namespace eval ::RootChild {variable v ROOT}
namespace eval ::N {
    namespace eval Child {variable v LOCAL}
    lappend ::__query_rows [list bare-global [namespace which -variable g]]
    lappend ::__query_rows [list relative-root [namespace which -variable RootChild::v]]
    lappend ::__query_rows [list relative-local [namespace which -variable Child::v]]
    lappend ::__query_rows [list absolute-root [namespace which -variable ::RootChild::v]]
    lappend ::__query_rows [list absent [namespace which -variable missing]]
    variable g
    lappend ::__query_rows [list declaration-shell [namespace which -variable g]]
    proc query {} {
        set local LOCAL
        global g
        lappend ::__query_rows [list procedure-local [namespace which -variable local]]
        lappend ::__query_rows [list procedure-global-link [namespace which -variable g]]
    }
    query
}
puts [join $::__query_rows \n]
join $::__query_rows \n
