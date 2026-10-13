# F5 product fixtures

Product fixtures test parsing, report generation, archive IO and flow decoding.
They are separate from measured appliance naming answers.
The [fixture index](index.json) lists every retained product archive/configuration
and its consumer path. Certificate fixtures use the shared
[runtime builder](../../../../scripts/dev/generate-certificate-ucs.py), following
[generation and verification instructions](../../../../rust/tcl-bigip-query/tests/fixtures/certificate-generation.md).

The report generator already uses two broad synthetic estates, shared between
Rust and Python. Their [provenance](../../../../rust/bigip-report-gen/python/tests/data/PROVENANCE.md)
and [archive builder](../../../../rust/bigip-report-gen/python/tests/data/build_ucs.py)
define the source and packaging. Keep these device identities separate: merging
their Common objects would lose ownership and can create name collisions.

Maintain a small number of logical datasets:

| Dataset | Purpose |
| --- | --- |
| Estate A | Broad LTM/security/APM object and reporting coverage |
| Estate B | A second device for estate comparison and ownership |
| Specialist, only when needed | A feature or diagnostic structure absent from A/B |

SCF, UCS and qkview are representations of a dataset, not independently
maintained copies of the same configuration. Generate packaging derivatives
from their shared source. A qkview requires its diagnostic structure and
provenance; a renamed UCS is not qkview coverage.

Retain small explicit format controls for truncation, corruption, missing
members and distinct encryption envelopes. Generate before/after estates
from one source plus a patch. Keep actual format distinctions rather than
deduplicating by file size or superficial similarity.

The reviewed snapshot has nine named UCS files and two SCFs, with no named
`.qkview` fixture. These small product files remain at their consumers' paths;
their archive and encryption assertions remain separate from oracle evidence.
This migration removes the thousands of expanded oracle copies and does not
replace a product fixture's independent wire-format contract.
