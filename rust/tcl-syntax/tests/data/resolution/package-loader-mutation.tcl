proc unrelated {} {return OLD}
package ifneeded ProbeMutation 1.0 {
    rename unrelated {}
    proc unrelated {} {return NEW}
    package provide ProbeMutation 1.0
}
puts [unrelated]
puts [package require ProbeMutation]
puts [unrelated]
