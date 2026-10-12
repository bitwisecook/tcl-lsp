# Measured literal loader controls: nh20261012literal1

[ANSWERS.md](ANSWERS.md) and [records.json](records.json) retain 55
measurements: 15 reached operations and 40 configuration load rejections.
Five literal operations reached execution in each of the CLI, iApp
implementation and iCall contexts. All attempted TMM configurations were
rejected before their intended runtime lookups.

[reviewed-proofs.json](reviewed-proofs.json) selects those 15 reached
operations. They have setup completion 0, complete operation captures,
runtime cleanup completion 0 and owned-object absence checks. Loader
rejection remains separate from a caught runtime error or an unsupported
name-resolution operation.

[literal.py](literal.py) supplies authored template provenance. Exact
materialised operations and applied configuration are retained separately;
their hashes and literal source inclusion were checked. The measured literal
load behaviour does not replace the separately measured dynamic-command
controls or answer an original portable input.
