# VM entry control selector preflight

All four actual test targets build, but zero controls execute because the new evaluation-entry selector names the source filename rather than its Rust module. The actual selector uses exec::native_script. Full captured source/provider/dependency/artifact drift maps are zero. Correcting that selector retains the same source and all52 controls.
