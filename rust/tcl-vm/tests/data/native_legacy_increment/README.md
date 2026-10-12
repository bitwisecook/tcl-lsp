# Original-object increment observations

The six native engines provide 180 fixed command observations: fifteen original
current/amount shapes, both unshared and shared receivers. The probe records
primary types, resident-string presence, reference counts and integer payloads
before observing the command result string. It records the original shared
receiver after execution and whether the variable still owns that same object.

The result observer runs after those physical windows. The corpus supplies no
return-options or trace-order evidence. The manifest identifies the actual
source, headers, libraries, executables and observation hashes.

Tcl 8.4 uses a Long amount getter before receiver reads, duplicates a shared
receiver before current conversion, and retains Long versus Wide arithmetic.
Jim uses its original-object safe-expression amount getter before receiver
selection and converts the current object before its sharing decision. Modern
C Tcl uses its separate full-number increment protocol.

The supported native Tcl 8.4 library produces `-8` when its string updater
formats the minimum signed Wide value in the endpoint-overflow cases. The VM
retains the measured wrapped integer payload and explicitly refuses that
unselected string-updater behaviour. Those two observations test the refusal
boundary; they do not count as native result-string matches.
