# Fixture for #865 — a file that was opened (and showed problems) must keep its
# Problems / File-Explorer badge after its editor tab is closed.
#
# `y` is assigned but never read → W211, a stable diagnostic that proves the
# analyser ran and gives the retained badge something to carry.
proc foo {} {
    set y 1
}
