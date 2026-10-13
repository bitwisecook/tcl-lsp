# Braced source correspondence owner controls

The shared continuation/native-unit owners pass25 Syntax controls and6 Registry
word controls. `syntax.log` and `registry.log` retain the complete captured
commands' output. New mapping controls require exact original boundaries,
refuse transformed-continuation crossings and partial native units, and retain
Document/native-value channel differences. Existing Syntax source-byte controls
consume real C8.4–9.1/Jim oracle vectors; the original custom LF/CRLF measurements
are retained under [braced correspondence](../../braced-source-correspondence/).

`record.json` binds the two changed owners and supporting naming/manifests to
hashes matching the coordinated frozen Compiler snapshot. These commands did
not independently seal the entire dependency closure. This receipt establishes
owner checks, not compiler/LSP execution or a #2286 fix. The original W210
control remains failing in its separately retained Compiler capture. No full
repository gate or F5 behaviour is established.
