# Tcl execution evidence

One Markdown file per script that actually ran, with the environments it ran in
and what each one returned. The format is defined in
[tcl-execution-evidence.md](../../docs/design/contracts/tcl-execution-evidence.md).

BIG-IP files whose harness is shared link a [fixture](fixtures/README.md) and
record only what the case changes.

```sh
make check-tcl-evidence
python3 scripts/dev/extract-tcl-evidence.py evidence/tcl/<file>.md -o script.tcl
```
