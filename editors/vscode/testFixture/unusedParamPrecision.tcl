# W214 unused-parameter range precision.
# Each unused parameter must be squiggled on its own name, not on the
# whole proc definition.
proc f {unused x} {
    puts $x
}
proc g {aa bb} {
    return 0
}
