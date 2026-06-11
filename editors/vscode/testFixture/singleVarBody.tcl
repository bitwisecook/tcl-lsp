# FP-STY-14 fixture: single bare-variable body is a script reference (no W105).
# Each of these holds the script in a variable — bracing would be wrong.
eval $cmd
namespace eval :: $state(-command) $token
after 0 $coroName
proc $fakeName $arglist $body
# Composite / quoted bodies DO weave substitutions into an inline script and
# MUST still fire W105:
eval "do $script"
eval $cmd$args
