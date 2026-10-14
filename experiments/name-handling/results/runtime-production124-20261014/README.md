# Runtime allocator fixture module path

Zero semantic tests execute: three new allocator fixture references name Interp at the crate root instead of its actual interp module. Original provider probes pass and captured source/dependency/artifact drift maps are zero. The correction uses the existing actual module path, with no API exposure or assertion change.
