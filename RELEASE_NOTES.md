# v1.5.3

## Bug Fixes

- CI: install `uv` in the CLI, unified Tcl, AI, WASM, and Rust core zipapp
  build jobs so the Python environment can be synced before building. Without
  this, the v1.5.2 release pipeline failed in those jobs.
- CI: bump Node.js to `lts/*` for the VS Code extension and integration test
  jobs to track the latest LTS instead of the now-superseded Node 20.
