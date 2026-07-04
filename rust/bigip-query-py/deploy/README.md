# Hosting the report on GitHub Pages

`github-pages.yml` is a ready-to-use GitHub Actions workflow that builds the
interactive report from the committed sample UCS fixtures and publishes it to
GitHub Pages — a stable URL that always reflects the latest code.

It lives here (not under `.github/workflows/`) because it must be installed by
someone whose credentials carry the `workflow` scope; automated pushes from this
tooling are not allowed to create workflow files.

## One-time setup

1. **Add the workflow** — copy it into place and commit (from a clone with
   workflow permission):

   ```bash
   mkdir -p .github/workflows
   cp rust/bigip-query-py/deploy/github-pages.yml .github/workflows/
   git add .github/workflows/github-pages.yml && git commit -m "Add BIG-IP report Pages workflow" && git push
   ```

2. **Enable Pages** — on github.com (the mobile app doesn't expose this; use a
   browser, "Request desktop site" if needed): repo **Settings → Pages → Build
   and deployment → Source → "GitHub Actions"**.

3. **Run it** — it runs automatically on pushes to `main` that touch the report,
   or trigger it manually from the **Actions** tab → *Publish BIG-IP example
   report to GitHub Pages* → **Run workflow**.

The report is then served at `https://<owner>.github.io/<repo>/`.

## Notes

- **One Pages site per repo.** This workflow and the existing (manual-only)
  Compiler-Explorer `pages.yml` both target the single GitHub Pages site (shared
  `pages` concurrency group / `github-pages` environment); the most recent run
  is what's live. Keep one as the active publisher, or merge them if you want
  both (e.g. the report under a `/report/` subpath).
- **Environment branch policy.** The `github-pages` environment may restrict
  deployments to the default branch. Once this branch is merged to `main`, the
  push trigger deploys automatically; to deploy from a feature branch first,
  allow it under Settings → Environments → github-pages.
- **No wasm toolchain needed.** The Mermaid library and the wasm query engine
  are vendored in the repo, so CI only builds the PyO3 extension (Rust + Python
  + maturin). The extension targets the CPython stable ABI (`abi3`, 3.9 floor),
  so any CPython ≥ 3.9 on the runner can build a wheel that runs on 3.9+; this
  workflow happens to use 3.14. GitHub Pages imposes no strict CSP, so the
  hosted page is the *full* report — the in-browser query console works there too.
- Change the input configs by editing the `python -m f5report …` line in the
  workflow (e.g. point it at your own committed `bigip.conf` / `.ucs`).
