# E001 - missing dispatch word (registry subcommand ensembles + TclOO
# object method dispatch), plus the false-positive carve-outs a correct
# implementation must respect.

# TP: a subcommand-dispatch registry command invoked with no subcommand at
# all is a genuine arity error.
string

# FP carve-out: `history` alone defaults to `history info` per history(n) -
# unlike `string`/`dict`/`info`, a bare call is well-defined Tcl and must
# not fire E001.
history

# TN: `history` with a subcommand stays clean either way.
history clear

# TP: TclOO's per-object command dispatcher requires a method word before
# it attempts any method lookup - `$o` alone is a genuine arity error.
oo::class create Dog {
    method bark {} { return woof }
}
set o [Dog new]
$o

# TN: supplying the method keeps the dispatch clean.
$o bark

# FP carve-out: snit's generated dispatcher proc is a different mechanism
# this analyser does not model precisely enough to assume it shares
# TclOO's unconditional "wrong # args" behaviour on a bare call.
snit::type Cat {
    method meow {} { return meow }
}
Cat c
$c

# TP (issue #1200): a bare TclOO object command produced by command
# substitution fails "wrong # args" before any method lookup - same
# zero-word dispatch failure as `$o` above, through a `[...]` head.
[Dog new]

# TN: with a method word the substitution-head dispatch is clean.
[Dog new] bark

# TP (issues #1143 / #1200): a handle returned by a `$var`-dispatched
# method and captured into a variable is typed through the object-type
# lattice - `$b bark` stays clean (no W307), and a bare `$b` is the same
# zero-word failure.
oo::class create Maker {
    method make {} { return [Dog new] }
}
set m [Maker new]
set b [$m make]
$b bark
$b
