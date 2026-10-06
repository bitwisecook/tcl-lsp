# Native try controls

`cases.tsv` contains fixed script inputs as byte hex. Each engine table records
case identity, evaluated completion code and exact result bytes. C Tcl 8.4 and
8.5 tables record command unavailability; executable structured-exception
comparisons use C Tcl 8.6, 9.0, 9.1 and Jim 0.84.

`probe.c` captures native result and return options before observer lookups.
The comparison fields are evaluated completion code and result only. The
manifest retains source, library, header, executable and log hashes. Raw
producer completion and private metadata require separate retained receipts;
root evaluation can settle a Return completion before this observation.
