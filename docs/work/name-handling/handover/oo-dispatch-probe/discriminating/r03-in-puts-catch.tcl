oo::class create C { method m {a} {} }
C create o
puts [catch {o m} msg]
