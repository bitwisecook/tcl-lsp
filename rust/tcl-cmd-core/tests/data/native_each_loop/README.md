# Generic native each loops

`original` contains 60 direct original-object command completions and 189 physical observation windows. `shimmer` contains 60 completions and 217 windows. Each producer calls the actual generic registered command through native object-vector evaluation on C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0, and Jim 0.84.

`R` records case, completion code, and result bytes in hexadecimal. `S` records case, observation sequence, and window; then the original variable root, value root, first original name, first original member, body, followed variable cell, member identity equality, and interpreter result. Each object field contains the actual primary type, reference count, resident-string presence, and List backing reference count. A backing count of -1 denotes a field unavailable from that native engine or object type.

Before, write, body, and after observations precede result-string observation. The C write callback reads a defined variable without installed read traces. The shimmer producer retains one explicitly declared external reference on its original first name and member, then invokes the native character-length operation on the original roots during the first body. Empty values never enter or parse the malformed body. Tcl 8.4 and 8.5 lmap observations retain the actual absent-command completion.
