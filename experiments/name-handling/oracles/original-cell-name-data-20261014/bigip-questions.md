# BIG-IP original variable-name observations

Run the exact corrected/probe.tcl with the actual accessible F5 Tcl interpreter.
Retain the original unmodified probe, raw stdout/stderr, completion status,
BIG-IP/TMM/interpreter version and the actual execution surface. If a surface
requires a wrapper, retain that wrapper separately and state its event/frame.
Do not substitute C Tcl or Jim observations for these answers.

Does the corrected probe complete and produce each of the ten captured C/Jim
rows? In particular, do literal empty and dollar/bracket names, literal array
keys, foreach/catch destination names and the parsed expression reference keep
the same name data? Does the dynamic missing-name case return catch code 1?
If a surface refuses a command, retain the exact refusal and leave unreached
observations unanswered. Write answers and raw captures into this branch under
this oracle directory without changing the probe.
