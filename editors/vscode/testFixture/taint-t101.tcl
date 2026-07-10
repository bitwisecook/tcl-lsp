# Fixture for the T101 (tainted data into `puts`) VS Code extension test.
#
# `gets stdin` is a taint source; the untransformed value flowing straight
# into `puts` is the T101 output-injection sink the quick fix should offer
# to sanitise (strip CR/LF) before printing.

set x [gets stdin]
puts $x
