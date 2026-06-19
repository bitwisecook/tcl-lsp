"""Front-end-behaviour contract tests for the command and graph registries.

These tests drive the real ``tcl`` and ``f5`` front-ends — and, for
presence and structural invariants, read the registry directly — asserting
behaviour against the language-agnostic golden CSVs under
``tests/baselines/registry/``.  The fixtures are the registry shape
contract; a Rust front-end re-implementing the ``command-info`` /
``event-info`` verbs is validated against the same files.
"""
