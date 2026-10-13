# Indexed F5 capture store

`catalogue.json.gz` maps historical logical paths to content digests, byte
lengths and file modes. `artefacts.zip` stores each distinct body once, under
its SHA-256. ZIP compression and whole-file deduplication preserve every
retained byte. No archive extraction is needed for proof validation.

The small [compile-time input index](compile-time-inputs.json) identifies the
byte-exact leaves still expanded for existing Rust `include_str!` consumers.
They retain their historical paths so this evidence migration needs no Rust
implementation change. They are build fixtures, not a second expanded run tree.

The capture store contains the original source/configuration, client and
server actions, queries, stdout/stderr, logs, ordered drivers, decoding
programs, cleanup receipts and reports. Empty captured files remain distinct
from missing captures through their catalogue entries and enclosing receipts.

Use `materialise.py read --path PATH` to inspect a body. Add `--json-pointer`
to select a structured measurement without printing an entire run record.
The corresponding question JSON provides the exact path and selector.

Run views keep only the question-relevant fields. Provider, raw-evidence and
cleanup descriptors are stored once by reference in `shared.json.gz`.
Full original records, raw chunks and contradictory or incomplete observations
remain in this store so another decoder can check the interpretation.

Compression does not sanitise a file. Existing redaction notices and the
distinction between historical execution and retained projected hashes remain
part of each receipt. Regenerated certificates cannot decrypt an old TLS
session: historical payload decoding needs its recorded session material or
an explicit declaration that only sanitised decoded results remain available.
Do not retain credentials, certificate/private-key bodies or TLS secrets in
repository evidence. Keep generation/verification recipes and, where needed,
protected external decoding references instead.

## Updating the store

Pack reviewed, already sanitised captures into a new store. Existing logical
paths are replaced by the supplied reviewed bytes; unreferenced old bodies are
omitted. The tool verifies lengths and archive contents before reporting success.
It does not sanitise, remove source files or overwrite the active store.

```sh
python3 scripts/dev/pack-oracle-evidence.py --source REVIEWED_SOURCE_ROOT \
  --prefix experiments/oracles/f5/runs/generated \
  --previous experiments/oracles/f5/captures --output /tmp/f5-new-store
```

Review the changed catalogue and answer scopes, replace the store pair together,
then run `scripts/dev/rebuild-f5-evidence.py`. Remove expanded reviewed captures
only after their logical paths can be read from the new store.
