package require tcltest
namespace import ::tcltest::*

testConstraint needsNet 1
customMatch approx ::approxEq

test case-1 {a test case} -body {
    set x 1
} -result 1
