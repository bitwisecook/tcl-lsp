# Measured broad cases and access interruption: nh20261012broad1

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 792
measurements: 705 observed, 78 unreached and nine infrastructure failures.
The stock interpreters reach 57 cases in 8.4 and 59 in 8.5; CLI, iApp
implementation and iCall each reach 55. RULE_INIT reaches 57 cases on all
four units. Procedure captures reach 49 cases on all four units before the
access interruption. Failed setup remains unreached.

[reviewed-proofs.json](reviewed-proofs.json) selects 387 representatives.
Their runtime cleanup completion is 0. Retained hosted post-delete receipts
find owned configuration objects absent. Stock observations have no tmsh
object-absence receipt. Source provenance is the authored
[broader_cases.py](broader_cases.py) template, with materialised Tcl and
case metadata retained separately; no exact portable input is claimed.

[capture-interrupted.json](capture-interrupted.json) records the SSH
authentication failure before the last procedure group's driver/evidence
capture completed. The remaining nine procedure cases have no authoritative
runtime capture; traffic and APL were not attempted in this run. The later
[recovery-absence.json](recovery-absence.json) and stderr independently find
the interrupted job's exact rule absent. This resolves its pending absence
check without supplying missing ownership, configuration or runtime evidence.
The later [final-runtime.stdout](final-runtime.stdout) is retained as a
supplementary snapshot. `MANIFEST.sha256` seals the retained files.
