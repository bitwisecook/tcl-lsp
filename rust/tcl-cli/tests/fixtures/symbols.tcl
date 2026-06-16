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

when HTTP_REQUEST {
    log local0. "request"
}
when HTTP_RESPONSE {
    log local0. "response"
}
