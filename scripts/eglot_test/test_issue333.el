;; -*- lexical-binding: t -*-
;;; test_issue333.el — headless eglot reproduction harness for issue #333
;;
;; Drives eglot 1.23 (GNU ELPA) against `uv run python -m lsp` from a
;; batch Emacs.  For each defined SCENARIO:
;;   1. open a Tcl file
;;   2. make a series of edits (each triggers didChange, possibly without
;;      waiting for the resulting semantic-tokens response)
;;   3. capture eglot's `face' property at every position (snapshot A)
;;   4. kill+reopen the file (sends didClose+didOpen → fresh /full request)
;;   5. capture face property again (snapshot B)
;;   6. diff A vs B; mismatch ⇒ bug reproduced.
;;
;; Exits 0 if all scenarios pass, 1 if any reproduces a bug, 2 on error.

(require 'eglot)
(require 'tcl)
(require 'cl-lib)

(unless (fboundp 'eglot-semantic-tokens-mode)
  (princ "FATAL: this eglot has no eglot-semantic-tokens-mode (need >= 1.23).\n")
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

(setq eglot-server-programs
      `((tcl-mode . ("uv" "run" "--directory" ,t333-repo
                     "--no-dev" "python" "-m" "lsp"))))


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

(defun t333-paint-from-cache ()
  "Force eglot's semtok painter to run with the current cached data.
Needed in batch mode because font-lock-mode is off and the keyword
form may not get re-invoked automatically after an async response."
  (when (and (cl-getf eglot--semtok-state :data)
             (fboundp 'eglot--semtok-font-lock-1))
    (eglot--semtok-font-lock-1
     (point-min) (point-max)
     (cl-getf eglot--semtok-state :data))))

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
  (let ((n (min (length a) (length b))))
    (cl-loop for i from 0 below n
             for ca = (nth i a) for cb = (nth i b)
             unless (equal (caddr ca) (caddr cb))
             collect (list i (cadr ca) (caddr ca) (caddr cb)))))

(defun t333-face-list (face-prop)
  "Return FACE-PROP normalized as a list of face symbols."
  (cond ((null face-prop) nil)
        ((symbolp face-prop) (list face-prop))
        ((and (consp face-prop) (keywordp (car face-prop))) nil) ; plist
        ((listp face-prop) face-prop)
        (t nil)))

(defun t333-find-accumulated-faces (snap)
  "Return positions in SNAP whose `face' contains a duplicated eglot face.
This catches the upstream eglot bug where `eglot--semtok-font-lock-1'
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


;;; -----------------------------------------------------------------------
;;; Scenarios

;; Each scenario is a plist:
;;   :name STRING       - short identifier
;;   :initial STRING    - file content before edits
;;   :edits FUNCTION    - (lambda () ...) makes edits in current buffer.
;;                        May call (t333-pump 0.5) between for partial waits,
;;                        or omit waits entirely to send rapid-fire didChange.
(defvar t333-scenarios
  (list

   `(:name "rename-only"
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
     :initial ,(concat "set a 1\nset b 2\nset c 3\n")
     :edits ,(lambda ()
               (goto-char (point-min))
               (insert "# leading comment\n")
               (t333-pump 1)))

   `(:name "delete-line-middle"
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
     :initial ,(concat
                "proc f {a b} {\n"
                "    set x 1\n"
                "    set y 2\n"
                "    return [expr {$x + $y + $a + $b}]\n"
                "}\n")
     :edits ,(lambda ()
               ;; A burst of edits without waiting between — exercises
               ;; eglot's request coalescing.
               (goto-char (point-min))
               (insert "# 1\n")
               (insert "# 2\n")
               (search-forward "set y 2")
               (replace-match "set y 22")
               (search-forward "$x + $y")
               (replace-match "$x * $y")
               (t333-pump 1)))

   `(:name "user-issue-code"
     ;; The actual code from the screenshots.
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
               ;; Reproduce a sequence the user might do.
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
               ;; Type-by-character into the proc body.
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
          ;; Look for our server's [timing] semanticTokens log lines in
          ;; the events buffer (window/logMessage notifications).
          (events-buf (cl-find-if
                       (lambda (b) (string-match-p "EVENTS for" (buffer-name b)))
                       (buffer-list))))
      (when events-buf
        (let ((tokens 0) (deltas 0))
          (with-current-buffer events-buf
            (save-excursion
              (goto-char (point-min))
              (while (re-search-forward "semanticTokens/full[^/]" nil t)
                (cl-incf tokens))
              (goto-char (point-min))
              (while (re-search-forward "semanticTokens/full/delta" nil t)
                (cl-incf deltas))))
          (princ (format "  protocol: %d /full requests, %d /full/delta requests\n"
                         tokens deltas))))
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
          ;; Bug-mode A: diff between post-edit and fresh reload.
          (when diff
            (princ (format "  FAIL [diff]: %d positions differ\n" (length diff)))
            (cl-loop for d in diff for k from 0 below 12
                     do (princ (format "    pos=%4d char=%c edit=%S  reload=%S\n"
                                       (nth 0 d) (nth 1 d) (nth 2 d) (nth 3 d)))))
          ;; Bug-mode B (issue #333): same eglot-semantic-* face appears
          ;; multiple times on a single character within one snapshot.
          ;; This catches the upstream eglot painter accumulation bug.
          (when accum-edited
            (princ (format "  FAIL [accum-edited]: %d positions have duplicated eglot-semantic-* faces\n"
                           (length accum-edited)))
            (cl-loop for a in accum-edited for k from 0 below 6
                     do (princ (format "    pos=%4d char=%c faces=%S\n"
                                       (nth 0 a) (nth 1 a) (nth 2 a)))))
          (when accum-reload
            (princ (format "  FAIL [accum-reload]: %d positions have duplicated eglot-semantic-* faces (in fresh-open buffer!)\n"
                           (length accum-reload))))
          (cond
           ((and (null diff) (null accum-edited) (null accum-reload))
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
for an end-user-facing example."
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
      ;; Capture face state immediately after the natural fontify.
      ;; `font-lock-flush' would re-unfontify, so we don't call it
      ;; here — we want to see what happens when the painter is
      ;; invoked twice consecutively, which is the real-world flow.
      (let ((before-second-paint
             (cl-loop for pos from (point-min) below (point-max)
                      collect (cons pos (get-text-property pos 'face)))))
        ;; Second paint, no unfontify in between.
        (eglot--semtok-font-lock-1 (point-min) (point-max) data)
        (let* ((snap (cl-loop for pos from (point-min) below (point-max)
                              collect (list pos
                                            (char-after pos)
                                            (get-text-property pos 'face))))
               (accum (t333-find-accumulated-faces snap))
               ;; Did the buffer's face state change between paints?
               ;; If the painter were idempotent, it wouldn't.
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

(defun t333-main ()
  (let* ((scenario-results
          (mapcar (lambda (sc) (cons (plist-get sc :name)
                                      (t333-run-scenario sc)))
                  t333-scenarios))
         ;; Painter-accumulation is informational: it deterministically
         ;; reproduces an upstream eglot bug we cannot fix from the
         ;; server side.  We expect it to FAIL on every eglot version up
         ;; to and including the current GNU ELPA release; if it ever
         ;; PASSes we'd want to know (and probably remove this XFAIL).
         (accum-pass (t333-painter-accumulation-test))
         (any-fail (cl-some (lambda (r) (not (cdr r))) scenario-results)))
    (princ "\n========== summary ==========\n")
    (dolist (r scenario-results)
      (princ (format "  %-25s %s\n" (car r)
                     (if (cdr r) "PASS" "FAIL"))))
    (princ (format "  %-25s %s\n" "painter-accumulation"
                   (cond (accum-pass "PASS (eglot bug appears fixed!)")
                         (t "XFAIL (known upstream eglot bug, see issue #333)"))))
    (kill-emacs (if any-fail 1 0))))

(condition-case err
    (t333-main)
  (error
   (princ (format "ERROR: %S\n" err))
   (kill-emacs 2)))
