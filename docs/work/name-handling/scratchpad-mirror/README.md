# Scratchpad mirror

Copy (2026-10-22T13:00Z) of the coordinator session's scratchpad directories that agent notes and outstanding files refer to as `scratchpad/<dir>/...`. Paths here match those references. Agents in other sessions should read these files here; the original scratchpad is local to the coordinator's container.

- Files over 5 MB are not copied; they are listed in `.skipped-large.txt`. Binary files are listed in `.skipped-binary.txt`.
- `compiler-names/g1-command_binding.log` was a 2.1 GB runaway log. It was replaced by `compiler-names/g1-command_binding.trunc.log`: its first and last 2 MB, plus its test, result and panic lines.
- Some directories that older notes refer to (`agents`, `triage`, `cp09`, `merge`, `sol40`, `sol42`, `base`) are not in the scratchpad, so they are not copied here.
