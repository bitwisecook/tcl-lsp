# ParseFormatArgs active-call literal-pool lifetime

Can the actual C86 Runtime ParseFormatArgs invocation preserve its pool-owned defaults and conflict literal when its reached numeric ABI callback deletes all three implemented clock registrations?

Selected command deletion consumes its once-only delete callback during active dispatch. The command allocation remains retained independently. The three registrations each hold one Rc of the shared pool; those registration references therefore do not ensure pool lifetime through a getter callback.

The repair takes a separate call Rc at ParseFormatArgs entry and uses that same pool through success, command error and conflict publication. It does not alter registration ownership, deletion callback timing, Tcl object reference ownership or native getter semantics.

Two fixed original-object argument controls exercise conflict (`0 -gmt 1 -timezone :UTC`) and success (`0 -gmt 1`). The existing EffectHost delegates the genuine numeric environment and deletes all three real registrations from its reached ABI callback. Each control starts in a fresh C86 interpreter without a prior ParseFormatArgs result retaining literal children. They check the reached callback, removed registrations, exact result/error, live received objects and expected remaining receiver references.

These are Rust Runtime lifetime controls. They establish no C-native callback chronology, seven-worker pool closure, C9/Jim/F5 behavior or native private-layout conformance. Source excerpts are in source/; compile and execution records must be recorded separately when run.
