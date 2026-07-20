# Fixture for the qualified-namespace-variable go-to-definition regression
# (differential audit against tcllib's defer.tcl / uri.tcl): a fully
# `::`-qualified variable reference like `$::simple::v` never resolved at
# all — go-to-definition, hover, references, and rename all share the one
# scope-chain lookup that only special-cased bare (unqualified) names.
namespace eval ::simple {
    variable v "hello"
}

puts $::simple::v
