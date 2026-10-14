proc New {} {catch {interp delete vectorCoreChild}; interp create vectorCoreChild; interp eval vectorCoreChild {set child VALUE}}
New
puts [interp eval vectorCoreChild {set child}]
interp delete vectorCoreChild
puts [catch {interp eval vectorCoreChild {set child}}]
