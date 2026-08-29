# Security

## Reporting a vulnerability

Please report security issues privately via [GitHub Security
Advisories](https://github.com/bitwisecook/tcl-lsp/security/advisories/new)
rather than opening a public issue.  We aim to acknowledge within
72 hours.

## Supply-chain posture

This section documents the repo-side controls that the CI workflows
assume are in place.  Workflow-level controls (SHA pinning, attestation,
checksum verification, least-privilege `GITHUB_TOKEN`, expression-
injection guards, Dependabot) live in `.github/`; everything below is
configured in GitHub repo settings and must be checked manually.

### Required repo settings

These cannot be set from a workflow.  Verify on every audit pass.

**Settings → Actions → General → Workflow permissions**

- [x] `Read repository contents and packages permissions` (NOT "Read and
      write").  Each workflow then explicitly opts in to writes via its
      own `permissions:` block.  Defence in depth: a workflow that
      forgets to declare `permissions:` should default to read-only.
- [x] `Allow GitHub Actions to create and approve pull requests` —
      **off**.  No workflow in this repo needs this; leaving it on lets
      a compromised workflow self-approve a PR.

**Settings → Actions → General → Fork pull request workflows from
outside collaborators**

- [x] At minimum, `Require approval for first-time contributors who are
      new to GitHub`.  Prefer `Require approval for all outside
      collaborators` if release cadence allows.  This is the gate that
      stops a one-line malicious PR from running CI under your token.

**Settings → Rules → Rulesets → Branch protection for `rust`**

- [x] `Require a pull request before merging` — on
- [x] `Require approvals` — at least 1
- [x] `Dismiss stale pull request approvals when new commits are pushed`
      — on
- [x] `Require review from Code Owners` — on (uses `.github/CODEOWNERS`)
- [x] `Require status checks to pass before merging` — on, with
      `pr-gate` (from `.github/workflows/ci.yml`) listed as required
- [x] `Require branches to be up to date before merging` — on
- [x] `Require signed commits` — on
- [x] `Do not allow bypassing the above settings` — on
- [x] `Allow force pushes` — off
- [x] `Allow deletions` — off

**Settings → Code security → Dependabot**

- [x] `Dependabot alerts` — on
- [x] `Dependabot security updates` — on
- [x] `Grouped security updates` — on (matches the grouping in
      `.github/dependabot.yml`)

**Settings → Code security → Secret scanning**

- [x] `Secret scanning` — on
- [x] `Push protection` — on

**Settings → Code security → Code scanning**

- [x] CodeQL `default` setup OR the workflow-mode setup pointing at
      `.github/workflows/codeql.yml`

### Verifying release artefacts

Each release asset is signed and attested:

- `SHA256SUMS` + `SHA256SUMS.cosign.bundle` — cosign keyless OIDC
  signature over the checksums file.  Verify with:

      cosign verify-blob \
        --bundle SHA256SUMS.cosign.bundle \
        --certificate-identity-regexp "https://github.com/bitwisecook/tcl-lsp/" \
        --certificate-oidc-issuer https://token.actions.githubusercontent.com \
        SHA256SUMS

- Every release artefact (install.sh, *.vsix, *.pyz, JetBrains/Sublime/
  Zed packages) carries a SLSA build-provenance attestation.  Verify
  with:

      gh attestation verify --owner bitwisecook <artefact>
