# ``expr`` literals with 0x / 0o / 0b prefixes must parse as hex, octal
# and binary respectively.  Regression for the codegen bug where
# ``_emit_literal`` did ``int(value)`` (no base arg) and silently
# fell through to ``0`` for every prefixed literal — so
# ``expr {0x10 + 1}`` returned ``1`` instead of ``17``.
if {[expr {0x10}] != 16} { error "hex 0x10 broke: [expr {0x10}]" }
if {[expr {0x10 + 1}] != 17} { error "hex add broke" }
if {[expr {0b1010 & 0b0110}] != 2} { error "binary AND broke" }
if {[expr {0o17}] != 15} { error "octal broke" }
# Plain decimals still work.
if {[expr {42}] != 42} { error "decimal broke" }
if {[expr {-7}] != -7} { error "neg decimal broke" }
return ok
