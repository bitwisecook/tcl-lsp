Draft refresh (source checks only):

    python /tmp/lsp-source-geometry-selection/generate_selection.py

After Root has successfully frozen the actual ordinary five-library build:

    python /tmp/lsp-source-geometry-selection/generate_selection.py --destination ACTUAL_DESTINATION --freeze --output ACTUAL_DESTINATION/selection.json

After Root has also captured actual post-build listings, those receipts may be checked/bound without executing any binary:

    python /tmp/lsp-source-geometry-selection/generate_selection.py --destination ACTUAL_DESTINATION --freeze --listings-dir ACTUAL_RESULTS --output ACTUAL_DESTINATION/selection.json

ACTUAL_DESTINATION and ACTUAL_RESULTS are explicit Root-supplied paths. There is no guessed destination, copied predecessor ELF or binary/list invocation in this generator. Original runtime guards and budgets remain unchanged. Root's runner must still bind the actual list invocation/exit/output and before/after current ELF hashes; a raw listing file is not execution acceptance.
