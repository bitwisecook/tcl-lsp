# FP-STY-17 fixture: a same-file proc/alias that shadows a registry ensemble
# command must suppress W001 on calls through the shadowed name; an
# unshadowed ensemble in the same file must still fire.
proc string {op args} {
    return "shadowed:$op"
}
string reverse hello
proc myInfo {op args} { return $op }
interp alias {} info {} myInfo
info bogus
# `dict` is untouched by either shadow above — a genuine unknown subcommand
# here must still fire W001 (the marker that analysis ran).
dict bogus a b
