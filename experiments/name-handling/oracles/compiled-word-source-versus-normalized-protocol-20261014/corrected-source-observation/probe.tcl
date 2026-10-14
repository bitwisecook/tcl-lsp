set raw {$(1+2)}
puts "RAW=$raw"
set missing_code [catch {set value "$(1+2)"} missing_result]
puts "MISSING_CODE=$missing_code"
puts "MISSING_RESULT=$missing_result"
set bind_code [catch {set {(1+2)} ARRAY} bind_result]
puts "BIND_CODE=$bind_code"
set bound_code [catch {set value "$(1+2)"} bound_result]
puts "BOUND_CODE=$bound_code"
puts "BOUND_RESULT=$bound_result"
set library_code [catch {info library} library]
puts "LIBRARY_CODE=$library_code"
puts "LIBRARY=$library"
