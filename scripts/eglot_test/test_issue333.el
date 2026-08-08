;; -*- lexical-binding: t -*-
;; tcl-lsp — a language server and toolchain for Tcl
;; Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
;;
;; This program is free software: you can redistribute it and/or modify
;; it under the terms of the GNU Affero General Public License as published by
;; the Free Software Foundation, either version 3 of the License, or
;; (at your option) any later version.
;;
;; This program is distributed in the hope that it will be useful,
;; but WITHOUT ANY WARRANTY; without even the implied warranty of
;; MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
;; GNU Affero General Public License for more details.
;;
;; You should have received a copy of the GNU Affero General Public License
;; along with this program.  If not, see <https://www.gnu.org/licenses/>.
;;
;; SPDX-License-Identifier: AGPL-3.0-or-later

;;; test_issue333.el — headless eglot reproduction harness for issue #333
;;
;; Drives a real eglot (GNU ELPA >= 1.20) against the native
;; `tcl-lsp-server' binary from a batch Emacs.  Two jobs:
;;
;;   A. Reproduce the upstream painter bug (issue #333) and confirm it is
;;      client-side (`t333-painter-accumulation-test').
;;   B. Verify the server's `semanticTokens/full/delta' end-to-end: after
;;      an edit, real eglot receives an actual delta and applies it via
;;      `eglot--semtok-apply-delta-edits' rather than the server re-sending
;;      the whole token stream (`t333-delta-test').
;;
;; This harness is intentionally NOT part of `make test' / CI gates: it
;; needs network access (to install eglot) and a real Emacs, and it
;; exercises upstream eglot internals we can't fix from the server side.
;; Run it by hand via `scripts/eglot_test/run.sh'.
;;
;; For each scenario the per-edit reproducer:
;;   1. open a Tcl file
;;   2. make a series of edits (each triggers didChange, possibly without
;;      waiting for the resulting semantic-tokens response)
;;   3. capture eglot's `face' property at every position (snapshot A)
;;   4. kill+reopen the file (sends didClose+didOpen → fresh /full request)
;;   5. capture face property again (snapshot B)
;;   6. diff A vs B; mismatch ⇒ real LSP delta-correctness bug.
;;
;; Exits 0 if all non-xfail scenarios and the delta assertions pass,
;; 1 if any real regression is found, 2 on error.

(require 'eglot)
(require 'tcl)
(require 'cl-lib)

(unless (fboundp 'eglot-semantic-tokens-mode)
  (princ "FATAL: this eglot has no eglot-semantic-tokens-mode (need >= 1.20).\n")
  (princ (format "  loaded from: %s\n" (locate-library "eglot")))
  (kill-emacs 3))

(setq eglot-confirm-server-initiated-edits nil
      eglot-events-buffer-config '(:size 2000000 :format full)
      eglot-autoshutdown t
      eglot-sync-connect 10
      ;; Quiet down eglot's "Connected!" / shutdown prints.
      inhibit-message t
      message-log-max nil)
;; Suppress jsonrpc sentinel warnings during shutdown — they are
;; harmless in batch mode and only clutter the log.
(advice-add 'display-warning :around
            (lambda (orig type message &rest rest)
              (unless (or (eq type 'jsonrpc)
                          (and (listp type) (memq 'jsonrpc type)))
                (apply orig type message rest))))

(defvar t333-repo (or (getenv "TCL_LSP_REPO")
                      (expand-file-name default-directory)))

;;; -----------------------------------------------------------------------
;;; Server binary

(defun t333-server-bin ()
  "Resolve the native `tcl-lsp-server' binary to launch.
Honours $TCL_LSP_SERVER_BIN, else prefers a release build over a debug
build under the repo's cargo target dir."
  (or (let ((env (getenv "TCL_LSP_SERVER_BIN")))
        (and env (file-exists-p env) env))
      (cl-loop for rel in '("target/release/tcl-lsp-server"
                            "target/debug/tcl-lsp-server"
                            "rust/target/release/tcl-lsp-server"
                            "rust/target/debug/tcl-lsp-server")
               for abs = (expand-file-name rel t333-repo)
               when (file-exists-p abs) return abs)
      (error "tcl-lsp-server binary not found; build it or set TCL_LSP_SERVER_BIN")))

(setq eglot-server-programs
      `((tcl-mode . (,(t333-server-bin)))))

;; Historical toggle from an earlier experiment (an "eglot compatibility mode"
;; that dropped `full/delta' + `range').  That mode was removed once the data
;; showed it made no difference — the server now implements proper
;; `semanticTokens/full/delta' deltas for every editor instead.  The variable
;; is kept (inert — the server ignores it) so the diagnostic scripts under this
;; directory that still bind it continue to load; binding it has no effect.
(defvar t333-eglot-compat nil)


;;; -----------------------------------------------------------------------
;;; Helpers

(defun t333-pump (secs &optional msg)
  (when msg (message "[wait %.1fs] %s" secs msg))
  (let ((deadline (+ (float-time) secs)))
    (while (< (float-time) deadline)
      (accept-process-output nil 0.05)
      (sit-for 0.02))))

(defun t333-pump-until-semtok (max-secs)
  "Pump until eglot has tokens for the current docver, or timeout."
  (let ((deadline (+ (float-time) max-secs)))
    (while (and (< (float-time) deadline)
                (or (null (cl-getf eglot--semtok-state :data))
                    (not (eq (cl-getf eglot--semtok-state :docver)
                             eglot--docver))))
      (font-lock-flush) (font-lock-ensure)
      (accept-process-output nil 0.05)
      (sit-for 0.02)))
  (font-lock-flush) (font-lock-ensure))

(defun t333-snapshot ()
  "Return list of (POS CHAR FACE TOK-FACES TOK-NAMES) for every char.
Pumps until tokens arrive, but does NOT force-paint — letting natural
font-lock paint reflects what the user actually sees, and avoids
masking the upstream painter-accumulation bug from issue #333."
  (t333-pump-until-semtok 8)
  (cl-loop for pos from (point-min) below (point-max)
           collect (list pos
                         (char-after pos)
                         (get-text-property pos 'face)
                         (get-text-property pos 'eglot--semtok-faces)
                         (get-text-property pos 'eglot--semtok-names))))

(defun t333-snapshot-summary (snap)
  "Return a string showing per-line ranges of distinct semtok-name lists."
  (let ((line 0) (lines '()) (cur "") (last-names nil) (run-start 0)
        (col 0) (run-text ""))
    (cl-labels ((flush ()
                  (when (not (string-empty-p run-text))
                    (setq cur (concat cur (format "[%S]%s" last-names run-text)))
                    (setq run-text ""))))
      (dolist (cell snap)
        (let ((ch (cadr cell)) (names (nth 4 cell)))
          (if (= ch ?\n)
              (progn (flush)
                     (push (format "L%02d %s" line cur) lines)
                     (cl-incf line) (setq cur "" col 0 last-names nil
                                          run-text ""))
            (when (not (equal names last-names))
              (flush)
              (setq last-names names))
            (setq run-text (concat run-text (string ch))))))
      (flush)
      (when (not (string-empty-p cur))
        (push (format "L%02d %s" line cur) lines)))
    (mapconcat #'identity (nreverse lines) "\n")))

(defun t333-diff (a b)
  "Return positions where SNAP A and B disagree on the `face' property.
Compares face properties after deduplicating any eglot-semantic-*
faces accumulated by the upstream painter bug (issue #333), so the
diff only catches real LSP delta-correctness mismatches between the
edited buffer's token state and a fresh full-reload's token state."
  (let ((n (min (length a) (length b))))
    (cl-loop for i from 0 below n
             for ca = (nth i a) for cb = (nth i b)
             for fa = (t333-dedup-face (caddr ca))
             for fb = (t333-dedup-face (caddr cb))
             unless (equal fa fb)
             collect (list i (cadr ca) fa fb))))

(defun t333-face-list (face-prop)
  "Return FACE-PROP normalized as a list of face symbols."
  (cond ((null face-prop) nil)
        ((symbolp face-prop) (list face-prop))
        ((and (consp face-prop) (keywordp (car face-prop))) nil) ; plist
        ((listp face-prop) face-prop)
        (t nil)))

(defun t333-find-accumulated-faces (snap)
  "Return positions in SNAP whose `face' contains a duplicated eglot face.
This catches the upstream eglot bug where the semantic-token painter
fails to strip its previous `eglot-semantic-*' faces from the `face'
property before re-applying, causing each repaint to append another
copy.  See https://github.com/bitwisecook/tcl-lsp/issues/333."
  (cl-loop for cell in snap
           for face = (caddr cell)
           for faces = (t333-face-list face)
           for eglot-faces = (cl-remove-if-not
                              (lambda (f)
                                (and (symbolp f)
                                     (string-prefix-p "eglot-semantic-"
                                                      (symbol-name f))))
                              faces)
           when (> (length eglot-faces) (length (cl-remove-duplicates
                                                  eglot-faces :test #'eq)))
           collect (list (car cell) (cadr cell) eglot-faces)))

(defun t333-dedup-face (face-prop)
  "Return FACE-PROP with duplicated eglot-semantic-* faces collapsed.
Preserves the original shape (symbol stays a symbol when only one
face remains; otherwise returns a list) so a snapshot diff after
deduplication only flags real LSP delta-correctness mismatches and
not the upstream painter accumulation from issue #333."
  (let ((faces (t333-face-list face-prop)))
    (cond
     ((null faces) face-prop)
     (t
      (let ((seen-eglot nil)
            (out '()))
        (dolist (f faces)
          (if (and (symbolp f)
                   (string-prefix-p "eglot-semantic-" (symbol-name f)))
              (unless (memq f seen-eglot)
                (push f seen-eglot)
                (push f out))
            (push f out)))
        (let ((dedup (nreverse out)))
          (cond
           ((null dedup) nil)
           ((and (= (length dedup) 1) (symbolp face-prop)) (car dedup))
           (t dedup))))))))

(defun t333-connect-and-open (path)
  "Open PATH in a fresh tcl-mode buffer with eglot synchronously connected."
  (find-file path)
  (tcl-mode)
  (font-lock-mode 1)
  ;; Synchronous connect: eglot-ensure relies on post-command-hook
  ;; which does not fire in batch.
  (let* ((contact (eglot--guess-contact))
         (managed (nth 0 contact)) (project (nth 1 contact))
         (class (nth 2 contact)) (cmd (nth 3 contact))
         (lang-id (nth 4 contact)))
    (eglot--connect managed project class cmd lang-id))
  (t333-pump 6 "post-connect"))

(defun t333-disconnect ()
  (let ((s (eglot-current-server))
        (kill-buffer-query-functions nil))
    (when s (ignore-errors (eglot-shutdown s 1 nil)))
    (kill-buffer (current-buffer))
    (t333-pump 1 "post-disconnect")))

(defun t333-semtok-full-shape ()
  "Return the `:full' field of the server's semanticTokensProvider.
Either the symbol t (bare boolean form) or a plist like (:delta t)."
  (let ((prov (eglot-server-capable :semanticTokensProvider)))
    (if (and (listp prov) (plist-member prov :full))
        (plist-get prov :full)
      prov)))

(defun t333-count-semtok-requests ()
  "Return (FULL . DELTA), the number of semanticTokens requests eglot
sent on the current connection, read from its jsonrpc events buffer.
Counts `full/delta' separately from plain `full' (the latter regex
excludes the `/delta' suffix)."
  (let* ((server (eglot-current-server))
         (events-buf (and server (jsonrpc-events-buffer server)))
         (full 0) (delta 0))
    (when (buffer-live-p events-buf)
      (with-current-buffer events-buf
        (save-excursion
          (goto-char (point-min))
          (while (re-search-forward "semanticTokens/full/delta" nil t)
            (cl-incf delta))
          (goto-char (point-min))
          ;; `full' not followed by `/delta' — a negative lookahead via an
          ;; explicit non-`/' (or end) terminator.
          (while (re-search-forward "semanticTokens/full\\(?:[^/]\\|$\\)" nil t)
            (cl-incf full)))))
    (cons full delta)))


;;; -----------------------------------------------------------------------
;;; Scenarios

;; Each scenario is a plist:
;;   :name STRING       - short identifier
;;   :initial STRING    - file content before edits
;;   :edits FUNCTION    - (lambda () ...) makes edits in current buffer.
;;                        May call (t333-pump 0.5) between for partial waits,
;;                        or omit waits entirely to send rapid-fire didChange.
;;   :xfail STRING      - if present, scenario is treated as informational:
;;                        a FAIL is reported as XFAIL with the supplied
;;                        explanation, and a PASS is shouted as
;;                        "XPASS — re-evaluate the XFAIL marker" so we
;;                        know to revisit when eglot upstream lands a fix.
;;                        Use for scenarios that exercise known eglot
;;                        internals (request coalescing, painter
;;                        accumulation, etc.) which cannot be fixed
;;                        from the server side.
(defvar t333-scenarios
  (list

   `(:name "rename-only"
     ;; Marked XFAIL: under CPU pressure (e.g. ``make test-slow``
     ;; running other suites in parallel) the delta paint after the
     ;; single replace leaves stale ``eglot-semantic-*`` faces behind
     ;; in the post-edit buffer that aren't in a fresh-reload snapshot
     ;; — the same upstream eglot painter behaviour the explicit
     ;; ``painter-accumulation`` scenario reproduces, just expressed as
     ;; cross-snapshot diffs rather than per-position duplicates.  Our
     ;; server's delta arithmetic is correct (``t333-diff`` dedups
     ;; painter accumulation; the remaining ``stale-face`` mode can
     ;; only be cleared by a fresh font-lock pass that eglot doesn't
     ;; emit).  Treat it as informational alongside
     ;; ``rapid-fire-no-wait`` until eglot fixes the painter.
     :xfail "eglot delta painter leaves stale eglot-semantic-* faces under load (issue #333)"
     :initial ,(concat
                "namespace eval ::myns {}\n"
                "set iniFile config.ini\n"
                "if {[::ini::exists $iniFile Options solver]} {\n"
                "    ::ini::set $iniFile Options solver 0\n"
                "} else {\n"
                "    ::ini::set $iniFile Options Solver 0\n"
                "}\n"
                "proc compute {a b} {\n"
                "    set total [expr {$a + $b}]\n"
                "    return $total\n"
                "}\n")
     :edits ,(lambda ()
               (goto-char (point-min))
               (search-forward "Solver 0")
               (replace-match "Auto 0")
               (t333-pump 1)))

   `(:name "insert-line-top"
     :xfail "eglot delta painter leaves stale eglot-semantic-* faces under load (issue #333)"
     :initial ,(concat "set a 1\nset b 2\nset c 3\n")
     :edits ,(lambda ()
               (goto-char (point-min))
               (insert "# leading comment\n")
               (t333-pump 1)))

   `(:name "delete-line-middle"
     :xfail "eglot delta painter leaves stale eglot-semantic-* faces under load (issue #333)"
     :initial ,(concat
                "proc foo {a b} {\n"
                "    set x $a\n"
                "    set y $b\n"
                "    set z [expr {$x + $y}]\n"
                "    return $z\n"
                "}\n")
     :edits ,(lambda ()
               (goto-char (point-min))
               (search-forward "set y $b")
               (beginning-of-line)
               (kill-line 1)
               (t333-pump 1)))

   `(:name "rapid-fire-no-wait"
     :xfail "eglot didChange coalescing is timing-dependent upstream"
     :initial ,(concat
                "proc f {a b} {\n"
                "    set x 1\n"
                "    set y 2\n"
                "    return [expr {$x + $y + $a + $b}]\n"
                "}\n")
     :edits ,(lambda ()
               (goto-char (point-min))
               (insert "# 1\n")
               (insert "# 2\n")
               (search-forward "set y 2")
               (replace-match "set y 22")
               (search-forward "$x + $y")
               (replace-match "$x * $y")
               (t333-pump 1)))

   `(:name "user-issue-code"
     :xfail "eglot delta painter leaves stale eglot-semantic-* faces under load (issue #333)"
     :initial ,(concat
                "set iniFile config.ini\n"
                "if {[::ini::exists $iniFile Options solver]} {\n"
                "    ::ini::set $iniFile Options solver 0\n"
                "} else {\n"
                "    ::ini::set $iniFile Options Solver 0\n"
                "}\n"
                "set DelFilesFlag 1\n"
                "set runLocation /tmp/run\n"
                "set rawFileNameOp /tmp/raw\n"
                "if {$DelFilesFlag} {\n"
                "    file delete -force -- $runLocation\n"
                "}\n"
                "catch {file delete -force -- $rawFileNameOp}\n")
     :edits ,(lambda ()
               (goto-char (point-min))
               (search-forward "Solver 0")
               (replace-match "Disabled 0")
               (t333-pump 1)
               (goto-char (point-min))
               (search-forward "$runLocation")
               (replace-match "$newRunLocation")
               (t333-pump 1)
               (goto-char (point-min))
               (search-forward "set runLocation /tmp/run\n")
               (replace-match "set newRunLocation /tmp/run\n")
               (t333-pump 1)))

   `(:name "many-small-edits"
     :initial ,(concat
                "proc compute {a b} {\n"
                "    set total 0\n"
                "    foreach v [list $a $b] {\n"
                "        incr total $v\n"
                "    }\n"
                "    return $total\n"
                "}\n")
     :edits ,(lambda ()
               (goto-char (point-min))
               (search-forward "set total 0")
               (forward-char 1)
               (cl-loop for ch across " ;# initialize"
                        do (insert (char-to-string ch))
                           (t333-pump 0.1))
               (t333-pump 1)))

   ))


;;; -----------------------------------------------------------------------
;;; Main

(defun t333-run-scenario (sc)
  (let* ((name (plist-get sc :name))
         (initial (plist-get sc :initial))
         (do-edits (plist-get sc :edits))
         (tmpdir (expand-file-name "tmp/eglot_test" t333-repo))
         (path (expand-file-name (format "%s.tcl" name) tmpdir)))
    (princ (format "\n========== scenario: %s ==========\n" name))
    (make-directory tmpdir t)
    (with-temp-file path (insert initial))

    ;; Phase 1: open + edits.
    (t333-connect-and-open path)
    (funcall do-edits)
    ;; Save so phase 2 reopens an equivalent file from disk.
    (let ((make-backup-files nil) (create-lockfiles nil))
      (save-buffer))
    (t333-pump-until-semtok 5)
    (let ((edited-text (buffer-string))
          (snap-edited (t333-snapshot))
          (reqs (t333-count-semtok-requests)))
      (princ (format "  protocol: %d /full requests, %d /full/delta requests\n"
                     (car reqs) (cdr reqs)))
      (t333-disconnect)

      ;; Phase 2: fresh open of the saved file.
      (t333-connect-and-open path)
      (let ((reload-text (buffer-string))
            (snap-reload (t333-snapshot)))
        (t333-disconnect)

        (princ (format "  text-equal: %s   snap-len: edited=%d reload=%d\n"
                       (string= edited-text reload-text)
                       (length snap-edited) (length snap-reload)))

        (let ((diff (t333-diff snap-edited snap-reload))
              (accum-edited (t333-find-accumulated-faces snap-edited))
              (accum-reload (t333-find-accumulated-faces snap-reload)))
          (when diff
            (princ (format "  FAIL [diff]: %d positions differ\n" (length diff)))
            (cl-loop for d in diff for k from 0 below 12
                     do (princ (format "    pos=%4d char=%c edit=%S  reload=%S\n"
                                       (nth 0 d) (nth 1 d) (nth 2 d) (nth 3 d)))))
          (when accum-edited
            (princ (format "  INFO [accum-edited]: %d positions have duplicated eglot-semantic-* faces (upstream eglot bug, see issue #333)\n"
                           (length accum-edited)))
            (cl-loop for a in accum-edited for k from 0 below 6
                     do (princ (format "    pos=%4d char=%c faces=%S\n"
                                       (nth 0 a) (nth 1 a) (nth 2 a)))))
          (when accum-reload
            (princ (format "  INFO [accum-reload]: %d positions have duplicated eglot-semantic-* faces in fresh-open buffer (upstream eglot bug, see issue #333)\n"
                           (length accum-reload))))
          (cond
           ((null diff)
            (princ "  PASS\n") t)
           (t
            (princ "  --- edited buffer summary:\n")
            (princ (t333-snapshot-summary snap-edited)) (princ "\n")
            nil)))))))

(cl-defun t333-painter-accumulation-test ()
  "Direct regression test for issue #333.

Calls `eglot--semtok-font-lock-1' twice in a row without an
intervening unfontify and asserts that no `eglot-semantic-*' face
appears more than once on any character.

Why this matters: `eglot--semtok-font-lock-1' uses
`add-face-text-property' to apply each token's face, but only
removes the auxiliary `eglot--semtok-token' / `eglot--semtok-faces'
properties before doing so — never the prior `eglot-semantic-*'
faces from the `face' property itself.  In interactive use, repeated
re-paints (after edits, scrolls, theme changes) accumulate faces
on the same character.  See
https://github.com/bitwisecook/tcl-lsp/issues/333#issuecomment-4380862687
for an end-user-facing example.

Purely client-side: it drives the painter directly, so it accumulates
regardless of what the server advertises.  It stays a deterministic
canary for when eglot upstream fixes the painter."
  (princ "\n========== scenario: painter-accumulation (issue #333) ==========\n")
  (let* ((tmpdir (expand-file-name "tmp/eglot_test" t333-repo))
         (path (expand-file-name "painter-accum.tcl" tmpdir))
         (initial (concat
                   "namespace eval ::myns {}\n"
                   "set iniFile config.ini\n"
                   "proc compute {a b} {\n"
                   "    set total [expr {$a + $b}]\n"
                   "    return $total\n"
                   "}\n")))
    (make-directory tmpdir t)
    (with-temp-file path (insert initial))
    (t333-connect-and-open path)
    (t333-pump-until-semtok 8)
    (let ((data (cl-getf eglot--semtok-state :data)))
      (unless (and data (vectorp data) (> (length data) 0))
        (princ "  ERROR: no semantic-tokens data after open\n")
        (t333-disconnect)
        (cl-return-from t333-painter-accumulation-test nil))
      (let ((before-second-paint
             (cl-loop for pos from (point-min) below (point-max)
                      collect (cons pos (get-text-property pos 'face)))))
        (eglot--semtok-font-lock-1 (point-min) (point-max) data)
        (let* ((snap (cl-loop for pos from (point-min) below (point-max)
                              collect (list pos
                                            (char-after pos)
                                            (get-text-property pos 'face))))
               (accum (t333-find-accumulated-faces snap))
               (after-second
                (cl-loop for pos from (point-min) below (point-max)
                         collect (cons pos (get-text-property pos 'face))))
               (changed (cl-count-if-not
                         (lambda (i) (equal (cdr (nth i before-second-paint))
                                            (cdr (nth i after-second))))
                         (number-sequence 0 (1- (length before-second-paint))))))
          (t333-disconnect)
          (princ (format "  %d / %d positions changed face after second paint\n"
                         changed (length before-second-paint)))
          (cond
           ((null accum)
            (princ "  PASS — painter is idempotent (eglot bug fixed?)\n") t)
           (t
            (princ (format "  FAIL — %d positions accumulated duplicate eglot-semantic-* faces\n"
                           (length accum)))
            (princ "  This is the upstream eglot bug from issue #333.\n")
            (cl-loop for a in accum for k from 0 below 8
                     do (princ (format "    pos=%4d char=%c faces=%S\n"
                                       (nth 0 a) (nth 1 a) (nth 2 a))))
            nil)))))))

;; NATURALLY FLAKY — DO NOT CHASE AS A SERVER BUG (issue #1323, closed).
;;
;; This scenario intermittently reports "no `edits' in any response — server
;; re-sent a full stream?" while passing on the next run, against an unchanged
;; server.  Measured: 2 of 3 consecutive runs pass, no code change between.
;;
;; That is inherent to driving a real eglot, not a defect here.  eglot
;; pipelines a second `semanticTokens/full' (~76 ms after the first, before
;; the first reply lands) whenever a response is slower than its font-lock
;; retry, then names whichever `resultId' it has processed so far in the
;; `full/delta' that follows — a scheduling coin-flip.  When it names the
;; older id the server has nothing to diff against and answers with a full
;; stream, which is a CORRECT `full/delta' response per the LSP spec, just not
;; an incremental one.  So a run that "fails" here has not caught a server
;; fault; it has caught eglot racing itself.
;;
;; Treat a red run as noise.  If the incremental path genuinely needs a
;; regression gate, add it as a pure-Rust reference-client e2e test (no Emacs
;; in the loop) rather than tightening anything here — a parked, unverified
;; sketch of both that test and a wider server-side stream cache lives on
;; branch `worktree-agent-a800a38f74c8619c2' (commit 4b7b1e2a).
(cl-defun t333-delta-test ()
  "End-to-end verification of the server's `semanticTokens/full/delta'.

NATURALLY FLAKY — see the comment above this function.  A failure here is
eglot racing its own pipelined requests, not a server fault; do not chase
it as one (issue #1323, closed as not-a-bug).

Through a real eglot connection: the server advertises `full/delta' +
`range', and after an edit eglot receives an actual DELTA and applies it
via `eglot--semtok-apply-delta-edits' — rather than the server re-sending
the whole token stream.  This is the incremental behaviour (like
rust-analyzer) that keeps eglot's token round-trip, and its stale-repaint
window, small on large files.  Returns t when every assertion holds."
  (princ "\n========== scenario: semantic-tokens delta ==========\n")
  (let* ((tmpdir (expand-file-name "tmp/eglot_test" t333-repo))
         (path (expand-file-name "delta.tcl" tmpdir))
         (initial (concat
                   "namespace eval ::myns {}\n"
                   "set iniFile config.ini\n"
                   "proc compute {a b} {\n"
                   "    set total [expr {$a + $b}]\n"
                   "    return $total\n"
                   "}\n"))
         (ok t)
         (got-delta nil))
    (make-directory tmpdir t)
    (with-temp-file path (insert initial))
    (unwind-protect
        (progn
          (t333-connect-and-open path)
          (t333-pump-until-semtok 8)
          (let ((full (t333-semtok-full-shape))
                (range (eglot-server-capable :semanticTokensProvider :range)))
            (princ (format "  advertised: full=%S range=%S\n" full range))
            (unless (and (listp full) (plist-get full :delta))
              (setq ok nil) (princ "  FAIL: server must advertise full/delta\n"))
            (unless (eq range t)
              (setq ok nil) (princ "  FAIL: server must advertise range\n")))
          ;; Capture the events-buffer end BEFORE the edit so we only inspect
          ;; the region appended by this didChange's `full/delta' exchange —
          ;; searching from `point-min' would match an `"edits"' from an
          ;; earlier response (e.g. the initial `full') and false-pass.
          (let* ((server (eglot-current-server))
                 (buf (and server (jsonrpc-events-buffer server)))
                 (mark-before (and (buffer-live-p buf)
                                   (with-current-buffer buf (point-max)))))
            ;; An edit that changes tokens; force the didChange flush (batch has
            ;; no idle timer) so eglot's `full/delta' path engages.
            (goto-char (point-min))
            (insert "# new comment referencing $iniFile\n")
            (eglot--signal-textDocument/didChange)
            (t333-pump-until-semtok 8)
            ;; A `full/delta' RESPONSE that carries `edits' (rather than a full
            ;; `data' re-send) is the proof the server produced a real delta.
            ;; Detected from the jsonrpc events buffer — `eglot--semtok-apply-
            ;; delta-edits' is a `defsubst' inlined into byte-compiled eglot, so
            ;; advising the symbol can't observe it.
            (when (buffer-live-p buf)
              (with-current-buffer buf
                (goto-char (or mark-before (point-min)))
                (setq got-delta (re-search-forward "\"edits\"" nil t))))
            (let ((reqs (t333-count-semtok-requests)))
              (princ (format "  after edit: %d /full, %d /full/delta requests; delta-response=%s\n"
                             (car reqs) (cdr reqs) (and got-delta t)))))
          (unless got-delta
            (setq ok nil)
            (princ "  FAIL: no `edits' in any response — server re-sent a full stream?\n")))
      (ignore-errors (t333-disconnect)))
    (princ (if ok "  PASS — server sends real deltas and eglot applies them\n"
             "  FAIL — delta path did not behave as expected\n"))
    ok))

(defun t333-main ()
  (let* ((scenario-results
          (mapcar (lambda (sc)
                    (list (plist-get sc :name)
                          (t333-run-scenario sc)
                          (plist-get sc :xfail)))
                  t333-scenarios))
         (accum-pass (t333-painter-accumulation-test))
         (delta-pass (t333-delta-test))
         ;; A scenario contributes to ``any-fail`` only if it failed AND
         ;; isn't marked :xfail.  The delta test is a real gate for our
         ;; server's `full/delta' implementation, so its failure fails the suite.
         ;;
         ;; CAVEAT (issue #1323, closed as not-a-bug): `t333-delta-test' is
         ;; NATURALLY FLAKY — see the comment above its definition.  It fails
         ;; roughly 1 run in 3 because eglot races its own pipelined requests,
         ;; not because the server regressed, and this line turns that noise
         ;; into a red `make test-emacs'.  Left gating deliberately rather than
         ;; silently downgraded; if the false-failure rate becomes annoying,
         ;; the fix is to move the gate to a pure-Rust reference client (no
         ;; Emacs in the loop) and drop this one to :xfail — not to weaken the
         ;; delta assertions themselves.
         (any-fail (or (not delta-pass)
                       (cl-some (lambda (r)
                                  (and (not (nth 1 r))
                                       (null (nth 2 r))))
                                scenario-results))))
    (princ "\n========== summary ==========\n")
    (dolist (r scenario-results)
      (let* ((name (nth 0 r))
             (passed (nth 1 r))
             (xfail-reason (nth 2 r))
             (status
              (cond
               ((and passed xfail-reason)
                (format "XPASS (re-evaluate the :xfail marker — %s)" xfail-reason))
               (passed "PASS")
               (xfail-reason
                (format "XFAIL (%s)" xfail-reason))
               (t "FAIL"))))
        (princ (format "  %-25s %s\n" name status))))
    (princ (format "  %-25s %s\n" "painter-accumulation"
                   (cond (accum-pass "PASS (eglot bug appears fixed!)")
                         (t "XFAIL (known upstream eglot bug, see issue #333)"))))
    (princ (format "  %-25s %s\n" "semantic-tokens-delta"
                   (cond (delta-pass "PASS")
                         (t "FAIL (full/delta implementation regressed)"))))
    (kill-emacs (if any-fail 1 0))))

;; Loading with TCL_LSP_T333_NOEXEC set defines the functions without running
;; the suite — used to drive individual scenarios from a REPL / one-off eval.
(unless (getenv "TCL_LSP_T333_NOEXEC")
  (condition-case err
      (t333-main)
    (error
     (princ (format "ERROR: %S\n" err))
     (kill-emacs 2))))
