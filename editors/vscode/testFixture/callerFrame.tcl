# Caller-frame variable fixtures.
# The callee spells the caller-frame name literally in its own body,
# so no call-site word carries it (tclsh 9.0.4: build returns W1).
proc NameProcess {arguments object} {
    upvar name name
    set name W1
}
proc build {} {
    NameProcess x obj
    set out $name
}
