# interp cancel on an idle child returns {}; an earlier caught error must not resurface.
set c [interp create]
puts "fresh-cancel=[catch {interp cancel $c} m] <$m>"
catch {error earlier}
puts "after-caught=[catch {interp cancel $c} m] <$m>"
interp delete $c
