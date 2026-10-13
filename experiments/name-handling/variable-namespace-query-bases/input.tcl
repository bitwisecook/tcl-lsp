puts [list PATCHLEVEL [info patchlevel]]
set ::g GLOBAL
namespace eval ::RootChild {variable v ROOT}
namespace eval ::N {
    namespace eval Child {variable v LOCAL}
    puts [list bare-global [namespace which -variable g]]
    puts [list relative-root [namespace which -variable RootChild::v]]
    puts [list relative-local [namespace which -variable Child::v]]
    puts [list absolute-root [namespace which -variable ::RootChild::v]]
    puts [list absent [namespace which -variable missing]]
    variable g
    puts [list declaration-shell [namespace which -variable g]]
    proc query {} {
        set local LOCAL
        global g
        puts [list procedure-local [namespace which -variable local]]
        puts [list procedure-global-link [namespace which -variable g]]
    }
    query
}
