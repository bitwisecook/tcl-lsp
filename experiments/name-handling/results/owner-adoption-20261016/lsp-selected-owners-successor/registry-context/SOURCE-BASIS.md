# Source basis

This is a partial overlay, not a worktree and not a main-source mutation. `base/` retains every exact beforeimage and `edit/` every afterimage; manifest.json binds each file with SHA-256.

The Registry source and two contract beforeimages are the exact reviewed RegistryV3 (3b33b8f5c5dd9436e57909fb646570df14330bc3693bd35c480761c4628acf37) afterimages under /tmp/registry-instance-method-provider-v3/edit, pending Root integration. Compiler and resolved_invocation.rs beforeimages were copied readonly from current /workspace/tcl-lsp. No main source, Git or Cargo command was performed. Apply only after the V3 predecessor, or union exact reviewed hunks against later Root changes.

Original test suffixes in all four changed Rust files are hashed in manifest.json and checked byte-identical. New test modules are appended. This source-only packet cannot advance execution or migration acceptance counts.
