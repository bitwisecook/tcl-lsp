# W111: Line exceeds 120 characters
# Source: SpiceGenTcl/src/generalClasses.tcl, SpiceGenTcl/examples/ngspice/advanced/monte_carlo.tcl
#
# Lines longer than 120 characters harm readability. The LSP flags them
# as a style warning. This is purely cosmetic — no runtime impact.
#
# Expected: tclsh runs without error.

# This line is intentionally longer than 120 characters to trigger W111 — it just keeps going and going and going and going and going until it is well past the limit
puts "long line test"

# Fixed version using line continuation:
set message "This line is broken up using continuation so that no single line \
    exceeds the 120-character limit"
puts $message

puts "W111 test complete"
