# Cutting a squashed branch from `work`

`work` history still carries ~10 GB of generated evidence that was deleted at
the tip. Deleting files does not shrink clones; replacing the history does.
`rust` history never touched those paths, so the squashed branch sits on top
of it and keeps `rust` history and release tags intact.

## When

Cut only when no other session or automation is pushing to `work`: stop the
five-minute repository sync and land or park in-flight work first. Anything
pushed to old `work` after the cut must be cherry-picked onto the new branch.

## Steps

1. Make the tip clean. `make check-tracked-size` must pass, and the tree
   should hold only source, tests, test inputs/outputs and minimised
   [execution evidence](../../design/contracts/tcl-execution-evidence.md).

2. Fetch full history without blobs, then list branches still carrying the
   bloat:

   ```sh
   git fetch --filter=blob:none --unshallow origin '+refs/heads/*:refs/remotes/origin/*'
   python3 scripts/dev/cut-squashed-branch.py audit
   ```

3. Cut and inspect locally. The parent is `origin/rust` when `work` already
   contains it, otherwise their merge-base:

   ```sh
   python3 scripts/dev/cut-squashed-branch.py cut --branch work-squashed
   git diff --stat origin/rust work-squashed | tail -1
   git diff --quiet origin/work work-squashed && echo "same tree"
   ```

4. Push it and measure a fresh clone of only that branch:

   ```sh
   git push origin work-squashed
   git clone --single-branch --branch work-squashed --no-tags https://github.com/bitwisecook/tcl-lsp /tmp/slim
   du -sh /tmp/slim/.git
   ```

5. Archive the old history outside the repository, e.g. as a release asset or
   in an artefact store, so nothing is lost:

   ```sh
   git bundle create work-history.bundle origin/work
   ```

6. Switch over. CI (`browser-symbol-ownership-diagnostic.yml`,
   `manual-lsp-diagnostics.yml`, `perf.yml`) and
   [validation-tools](validation-tools/) check `refs/heads/work`, so the
   simplest switch keeps the name and moves `work` onto the squashed commit:

   ```sh
   git push --force-with-lease=work:$(git rev-parse origin/work) origin work-squashed:work
   git push origin --delete work-squashed
   ```

   To keep the new name instead, update those references to it and delete
   `work`.

7. Retire every branch the audit listed. On 5 November 2026 these six
   carried the history; every other branch, including `rust`, was clean:
   `work`, `lsp-release-stop-2026-11-04`, `codex/dependency-sweep-work`,
   `codex/jetbrains-2026-3-work`, `codex/tcl-evidence-markdown` and
   `codex/f5-evidence-layout`. Rerun the audit at cut time. For each one,
   merge what is needed into the new branch first, then delete it. A branch,
   tag or open pull request that still points at old `work` history keeps
   those objects in every full clone. Confirm with a fresh mirror:
   `git clone --mirror … && git log --all --oneline -- docs/work/name-handling/handoff`
   must print nothing.

8. Everyone re-clones, or, after pushing or stashing local work, prunes and
   collects an existing clone:

   ```sh
   git fetch --prune origin && git reset --hard origin/work
   git reflog expire --expire=now --all && git gc --prune=now
   ```

GitHub keeps objects referenced by closed pull requests (`refs/pull/*`), so the
size GitHub reports may not drop without a request to GitHub Support. Clones
do not fetch those refs, so clone size drops regardless.
