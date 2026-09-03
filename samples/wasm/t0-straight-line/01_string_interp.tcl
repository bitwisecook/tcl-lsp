# T0: string interpolation and append. No control flow, no procs.
set name world
set greeting "hello, $name"
append greeting "!"
puts $greeting
puts [string length $greeting]
