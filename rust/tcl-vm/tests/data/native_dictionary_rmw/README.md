# Native dictionary publication controls

The fixed scripts exercise compiled local opcodes and generic dictionary
commands separately. Controls cover read mutation, read failure, alias
replacement, write mutation, write failure and ordinary operations. The native
result is captured by the original script before final cell observations;
the reporter captures evaluated completion before observer lookups.

Each engine table contains only complete native processes with exit status zero:
345 fixed evaluated code/result comparisons across C Tcl 8.4–9.1 and Jim 0.84.
C Tcl 8.4 records dictionary command absence. Jim records trace command absence
in trace controls and executes the ordinary dictionary controls.

The manifest also retains 15 signal-terminated compiled alias-replacement
controls. Their emitted result rows are diagnostic observations; they do not
establish complete native executions or a positive compatibility reference.
The manifest preserves every process status, executable, source and raw-log hash.

Dictionary transformation and variable publication have separate owners.
Generic C commands look up the written variable again before the write; local
opcodes retain the selected cell across read callbacks. Source-object, key and
member cache effects require the physical dictionary and scalar receipts.
