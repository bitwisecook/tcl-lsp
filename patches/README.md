# patches/

Changes that must be applied from a clone whose credentials carry the GitHub
`workflow` OAuth scope — this automation cannot push files under
`.github/workflows/`.

## `github-workflows.patch`

Brings `.github/workflows/` from the `main` baseline up to the intended state
for this branch:

- the `rust` development line's CI workflow updates (`ci.yml`, `rust-gate.yml`,
  `rust-lsp-e2e.yml`, and the removal of the superseded `pages.yml`), and
- the GitHub Pages workflow (`github-pages.yml`) extended to build and publish
  the **in-browser BIG-IP report generator** at `/bigip-report/` alongside the
  existing example report and compiler explorer (this mirrors
  `rust/bigip-report/py/deploy/github-pages.yml`, the committed source copy of
  that workflow, which *is* included in this branch).

This branch itself keeps `.github/workflows/` identical to `main` so it can be
pushed; apply the patch where the `workflow` scope is available:

```bash
git apply patches/github-workflows.patch
git add .github/workflows && git commit -m "ci: rust workflows + publish report app to /bigip-report/"
```

If you are merging this branch into the `rust` line (which already carries the
CI workflow updates), you only need the `github-pages.yml` hunk — the rest is
already present there.
