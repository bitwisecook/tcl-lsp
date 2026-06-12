"""Front-end-behaviour contract tests for the command and graph registries.

These tests drive the real ``tcl`` and ``f5`` front-ends and assert their
output against the language-agnostic golden fixtures under
``tests/baselines/registry/``.  The fixtures are the registry shape
contract; a Rust front-end re-implementing the ``registry-dump`` /
``command-info`` / ``event-info`` verbs is validated against the same
files.
"""
