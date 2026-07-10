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
