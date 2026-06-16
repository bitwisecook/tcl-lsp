# Symbol-extraction fixture for the `symbols` verb.
namespace eval ::app {
    variable count 0
    variable name "app"

    proc greet {who {greeting hello}} {
        set msg "$greeting, $who"
        return $msg
    }

    namespace eval ::app::util {
        proc clamp {value lo hi} {
            return $value
        }
    }
}

proc main {} {
    proc helper {x} { return $x }
    ::app::greet world
}

# Multi-variable loop bindings share one source token in the analyser;
# per-name spans keep them in declaration order (regression guard).
foreach {first second third} {1 2 3} {
    incr first
}
dict for {dkey dval} {x 1 y 2} {
    incr dkey
}

when HTTP_REQUEST {
    log local0. "request"
}
when HTTP_RESPONSE {
    log local0. "response"
}
