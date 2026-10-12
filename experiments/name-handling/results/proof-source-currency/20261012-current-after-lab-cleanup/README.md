# Current evidence currency after lab-data sanitisation

The retained checker ran once against frozen canonical/checker/renderer and
4,839 selected source/manifests/toolchain files. It failed with 1,217 errors:
315 implementation digests and 902 source-census digests. The selected hashes
were unchanged during that run; exact command, source snapshots and raw output
are retained in [record.json](record.json), [before.json](before.json),
[after.json](after.json) and [check.log](check.log).

The [census correction](census-renewal.json) renews only the 902 sanitised
retained files' current hashes and 892 current byte counts. All 31,022 other
entries and every path/classification/proof association are unchanged. No
historical execution hashes or implementation bindings were renewed. This
metadata correction is independently verified; a subsequent checker result
must be captured separately. Incoming F5 finish evidence is a separate subject.
