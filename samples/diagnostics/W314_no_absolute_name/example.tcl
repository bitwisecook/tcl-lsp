# A lone colon is an ordinary name character: this proc is real, callable
# bare (`: a`), but no absolute spelling reaches it (`:::` names the {} proc).
proc : args {
    return "hello"
}

# Everything inside a namespace named ":" is reachable only relatively.
namespace eval : {
    proc helper {} { }
}
