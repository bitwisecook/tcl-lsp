# Compiler selector preflight

The ordinary compiler test library builds successfully; zero tests execute because one required callback/configuration selector names the wrong Rust test module. Source-subset drift is zero. The following capture uses the same exact executable and an expected-source inventory, correcting only the selector to the actual original test.
