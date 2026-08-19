# A2: cross-spelling removal with old_style set, and vinfo/info rendering
# after partial removes with mixed op sets.
proc cb args {}
proc cb2 args {}

# Legacy install, modern remove (C masks TCL_TRACE_OLD_STYLE out of the match).
trace variable a rw cb
puts "a1: [trace info variable a] / [trace vinfo a]"
trace remove variable a {write read} cb
puts "a2: [trace info variable a] / [trace vinfo a]"

# Modern install, legacy remove.
trace add variable b {unset array} cb
puts "b1: [trace info variable b] / [trace vinfo b]"
trace vdelete b ua cb
puts "b2: [trace info variable b] / [trace vinfo b]"

# Mismatched op sets must NOT remove.
trace variable c rwu cb
trace vdelete c rw cb
puts "c1: [trace vinfo c]"
trace remove variable c {read write} cb
puts "c2: [trace vinfo c]"
trace remove variable c {unset write read} cb
puts "c3: [trace vinfo c]"

# Mixed op sets on one variable: vinfo after a partial remove.
trace variable d r cb
trace variable d w cb
trace variable d u cb
trace add variable d {array read write unset} cb2
puts "d1: [trace vinfo d]"
puts "d1i: [trace info variable d]"
trace vdelete d w cb
puts "d2: [trace vinfo d]"
trace remove variable d {unset read array write} cb2
puts "d3: [trace vinfo d]"
puts "d3i: [trace info variable d]"

# Wrong command prefix must not remove.
trace vdelete d r cbX
puts "d4: [trace vinfo d]"

# vinfo/info on an untraced and on a never-mentioned name.
puts "e1: [trace vinfo nosuch] / [trace info variable nosuch]"

# Element vs whole-array identity.
array set arr {}
trace variable arr(k) w cb
trace variable arr w cb2
puts "f1: [trace vinfo arr] / [trace vinfo arr(k)]"
trace vdelete arr(k) w cb
puts "f2: [trace vinfo arr] / [trace vinfo arr(k)]"
