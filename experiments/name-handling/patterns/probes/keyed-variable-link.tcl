namespace eval ::nh {
    set {state.a b(value)} original
    proc change {name} {
        upvar ::nh::$name state
        set state(value) changed
    }
}
::nh::change {state.a b}
set {::nh::state.a b(value)}

