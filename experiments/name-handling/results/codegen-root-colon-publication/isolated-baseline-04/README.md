# Isolated base: missing compile-time fixture

The independent jobs1/testdebug0 build refused a missing tracked Tcl fixture
referenced through concat!(CARGO_MANIFEST_DIR). No tests ran. All 10,206
pre-captured inputs remain unchanged. The sparse-input dependency gap is retained
with exact compiler error and source/tool identity. No passing or native-entry
acceptance is claimed. The next capture includes exact restored fixture bytes.
