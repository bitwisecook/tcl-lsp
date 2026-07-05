# Adversarial dynamic-dispatch snippets for the mro_eval experiment.
#
# Every `$obj method` here MUST resolve to ⊤ (abstain).  A confident
# wrong resolution on any of these is a correctness failure of the
# resolver — worse than abstaining.  The companion integration test
# `tests/mro_lattice_adversarial.rs` asserts abstention (with the
# expected ⊤ reason) for each shape.

oo::class create Animal {
    method speak {} { return "..." }
}
oo::class create Dog {
    superclass Animal
    method speak {} { return "woof" }
}
oo::class create Cat {
    superclass Animal
    method speak {} { return "meow" }
}

# 1. dynamic-assign: reassigned to a non-object between object binding
#    and dispatch — the class is not stable.
proc reassigned {} {
    set o [Dog new]
    set o "just a string"
    $o speak
}

# 2. control-flow join of two different classes — a finite set, but the
#    reassignment to different concretes must still be tracked, and any
#    literal contamination forces ⊤.
proc branch_join {flag} {
    if {$flag} {
        set o [Dog new]
    } else {
        set o [Cat new]
    }
    $o speak
}

# 3. factory-return: bound from a proc whose object class we don't model.
proc make {} { return [Dog new] }
proc via_factory {} {
    set o [make]
    $o speak
}

# 4. introspection: the class is a runtime value.
proc via_introspection {x} {
    set cls [info object class $x]
    set o [$cls new]
    $o speak
}

# 5. oo::copy: a copy of an unknown object.
proc via_copy {other} {
    set o [oo::copy $other]
    $o speak
}

# 6. per-object mixin: the receiver's method table is extended at runtime.
proc via_objdefine {} {
    set o [Dog new]
    oo::objdefine $o { method fetch {} { return stick } }
    $o fetch
}

# 7. bare parameter receiver: only interprocedural flow could bind it.
proc param_receiver {obj} {
    $obj speak
}
