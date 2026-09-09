#!/usr/bin/env python3
"""Generate minimal Tcl reproducers for the phi_can_undef exponential DFS.

Usage: gen.py <variant> <N> [--no-read] [--no-init] [--proc]
Writes the script to stdout.
"""
import sys

def flat_if(n):
    # merge phi operands: (prev phi, concrete def)  -> linear chain
    return "\n".join("if {$c%d} { set x %d }" % (i, i) for i in range(n))

def flat_if_else(n):
    return "\n".join("if {$c%d} { set x %d } else { set x -%d }" % (i, i, i) for i in range(n))

def nested_if(n):
    # each stage: outer merge phi = phi(prev, inner merge phi) -> 2 phi operands
    out = []
    for i in range(n):
        out.append("if {$c%d} {\n    if {$d%d} { set x %d }\n}" % (i, i, i))
    return "\n".join(out)

def nested_if3(n):
    out = []
    for i in range(n):
        out.append("if {$c%d} {\n    if {$d%d} {\n        if {$e%d} { set x %d }\n    }\n}" % (i, i, i, i))
    return "\n".join(out)

def switch_empty(n, arms=3):
    # arms-1 empty arms each carry the *previous* version on a distinct pred edge
    out = []
    for i in range(n):
        body = ["    a { set x %d }" % i]
        for j in range(arms - 1):
            body.append("    b%d { }" % j)
        out.append("switch -- $v%d {\n%s\n}" % (i, "\n".join(body)))
    return "\n".join(out)

def switch_empty5(n):
    return switch_empty(n, arms=5)

def if_empty_else(n):
    # if {c} {} else {}  -- neither arm writes; merge phi = phi(prev, prev) on 2 preds
    return "\n".join("if {$c%d} { incr q } else { incr r }" % (i,) for i in range(n))

def foreach_nested_if(n):
    inner = nested_if(n)
    return "foreach it $items {\n%s\n}" % inner

def foreach_flat_if(n):
    inner = flat_if(n)
    return "foreach it $items {\n%s\n}" % inner


def quartus_chain(n):
    """Right-leaning else-chain, Quartus nf_pma shape:
       if A { if B { set x } } else { if A2 { if B { set x } } else { ... } }"""
    def rec(i):
        if i >= n:
            return "if {$e%d} { set x %d }" % (n - 1, n - 1)
        return ("if {$c%d} {\n    if {$d%d} { set x %d }\n} else {\n%s\n}"
                % (i, i, i, "\n".join("    " + l for l in rec(i + 1).splitlines())))
    return rec(0)

def quartus_seq(n):
    """Sequence of small Quartus-shaped decision trees, each conditionally
       writing the SAME variable -- the multiplicative case."""
    out = []
    for i in range(n):
        out.append("if {$c%d} {\n    if {$d%d} { set x %d }\n} else {\n    if {$e%d} { set x -%d }\n}" % (i, i, i, i, i))
    return "\n".join(out)

VARIANTS = {
    "quartus_chain": quartus_chain,
    "quartus_seq": quartus_seq,
    "flat_if": flat_if,
    "flat_if_else": flat_if_else,
    "nested_if": nested_if,
    "nested_if3": nested_if3,
    "switch_empty": switch_empty,
    "switch_empty5": switch_empty5,
    "if_empty_else": if_empty_else,
    "foreach_nested_if": foreach_nested_if,
    "foreach_flat_if": foreach_flat_if,
}

def main():
    variant = sys.argv[1]
    n = int(sys.argv[2])
    flags = set(sys.argv[3:])
    read = "--no-read" not in flags
    init = "--no-init" not in flags
    proc = "--proc" in flags

    body = []
    if init:
        body.append("set x 0")
    body.append(VARIANTS[variant](n))
    if read:
        body.append("puts $x")
    core = "\n".join(body)

    pre = []
    # define the conditions / lists so nothing else is read-before-set
    pre.append("set q 0")
    pre.append("set r 0")
    pre.append("set items [lindex $argv 0]")
    for i in range(n):
        pre.append("set c%d [lindex $argv %d]" % (i, i))
        pre.append("set d%d [lindex $argv %d]" % (i, i))
        pre.append("set e%d [lindex $argv %d]" % (i, i))
        pre.append("set v%d [lindex $argv %d]" % (i, i))
    preamble = "\n".join(pre)

    if proc:
        indented = "\n".join("    " + l for l in core.splitlines())
        print(preamble)
        print("proc f {%s} {" % " ".join("c%d" % i for i in range(n)))
        print("    set q 0")
        print("    set r 0")
        print("    set items [lindex $argv 0]")
        for i in range(n):
            print("    set d%d [lindex $argv %d]" % (i, i))
            print("    set e%d [lindex $argv %d]" % (i, i))
            print("    set v%d [lindex $argv %d]" % (i, i))
        print(indented)
        print("}")
    else:
        print(preamble)
        print(core)

main()
