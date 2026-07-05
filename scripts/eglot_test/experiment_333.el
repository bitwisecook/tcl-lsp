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

;;; experiment_333.el — differential proof + perf feel for issue #333.
;;
;; Load the harness helpers (NOEXEC), generate a large Tcl file, and run the
;; SAME harsh edit sequence through real eglot twice — compat OFF and compat
;; ON — reporting (a) accumulated `eglot-semantic-*` faces at the end of each
;; run and (b) the timings eglot actually waits on (connect→first tokens, and
;; each edit→fresh tokens round trip).  Run:
;;
;;   TCL_LSP_REPO=$PWD TCL_LSP_SERVER_BIN=$PWD/target/release/tcl-lsp-server \
;;   TCL_LSP_T333_NOEXEC=1 emacs -Q -batch \
;;     -L tmp/elpa/eglot-1.24 -L tmp/elpa/jsonrpc-1.0.29 \
;;     -l scripts/eglot_test/test_issue333.el \
;;     -l scripts/eglot_test/experiment_333.el

(defun exp333-big-tcl (nprocs)
  "Return a large Tcl source with NPROCS small procs (~9 lines each)."
  (let ((s (list "namespace eval ::bench {\n    variable counter 0\n}\n\n")))
    (dotimes (i nprocs)
      (push (format (concat "# proc number %d\n"
                            "proc ::bench::step%d {a b} {\n"
                            "    set v%d [expr {$a + $b}]\n"
                            "    set msg \"step %d = $v%d\"\n"
                            "    if {$v%d > 10} {\n"
                            "        set v%d [expr {$v%d + 1}]\n"
                            "    }\n"
                            "    return $v%d\n"
                            "}\n\n")
                    i i i i i i i i i)
            s))
    (apply #'concat (nreverse s))))

(defun exp333-now () (float-time))

(defun exp333-wait-fresh-tokens (max-secs)
  "Block until eglot has token data for the CURRENT docver; return seconds
waited (or MAX-SECS on timeout).  This is the latency a user perceives
between an edit and refreshed highlighting."
  (let ((start (exp333-now))
        (deadline (+ (exp333-now) max-secs)))
    (while (and (< (exp333-now) deadline)
                (or (null (cl-getf eglot--semtok-state :data))
                    (not (eq (cl-getf eglot--semtok-state :docver)
                             eglot--docver))))
      (font-lock-flush) (font-lock-ensure)
      (accept-process-output nil 0.02)
      (sit-for 0.01))
    (font-lock-flush) (font-lock-ensure)
    (- (exp333-now) start)))

(defun exp333-accum-count ()
  "Count buffer positions carrying a duplicated eglot-semantic-* face."
  (length (t333-find-accumulated-faces
           (cl-loop for pos from (point-min) below (point-max)
                    collect (list pos (char-after pos)
                                  (get-text-property pos 'face))))))

(cl-defun exp333-run (compat nprocs)
  "Drive the harsh edit sequence with COMPAT (nil/t). Return a plist of
timings and the final accumulation count."
  (let* ((tmpdir (expand-file-name "tmp/eglot_test" t333-repo))
         (path (expand-file-name (format "exp-%s.tcl" (if compat "on" "off")) tmpdir))
         (src (exp333-big-tcl nprocs))
         (t333-eglot-compat compat)
         (edit-times '()))
    (make-directory tmpdir t)
    (with-temp-file path (insert src))
    ;; Connect + time first tokens.
    (find-file path) (tcl-mode) (font-lock-mode 1)
    (let* ((c (eglot--guess-contact))
           (t0 (exp333-now)))
      (eglot--connect (nth 0 c) (nth 1 c) (nth 2 c) (nth 3 c) (nth 4 c))
      (let ((connect-tokens (exp333-wait-fresh-tokens 60)))
        (let ((lines (count-lines (point-min) (point-max))))
          ;; Harsh sequence: rename a var near the top, middle and bottom, then
          ;; an edit-then-undo — each forced onto the wire and timed.
          (dolist (needle (list (format "set v1 " )
                                (format "set v%d " (/ nprocs 2))
                                (format "set v%d " (- nprocs 2))))
            (goto-char (point-min))
            (when (search-forward needle nil t)
              (replace-match (replace-regexp-in-string "set v" "set vv" needle) t t)
              (eglot--signal-textDocument/didChange)
              (push (exp333-wait-fresh-tokens 60) edit-times)))
          ;; edit-then-undo near the middle.
          (goto-char (point-min))
          (when (search-forward (format "step%d = " (/ nprocs 2)) nil t)
            (let ((p (match-beginning 0)))
              (goto-char p) (insert "X")
              (eglot--signal-textDocument/didChange)
              (push (exp333-wait-fresh-tokens 60) edit-times)
              (goto-char p) (delete-char 1)  ; undo
              (eglot--signal-textDocument/didChange)
              (push (exp333-wait-fresh-tokens 60) edit-times)))
          (let ((accum (exp333-accum-count)))
            (ignore-errors (eglot-shutdown (eglot-current-server) 1 nil))
            (let ((kill-buffer-query-functions nil)) (kill-buffer (current-buffer)))
            (list :compat compat :lines lines
                  :connect-tokens connect-tokens
                  :edit-times (nreverse edit-times)
                  :accum accum)))))))

(defun exp333-report (r)
  (let ((et (plist-get r :edit-times)))
    (princ (format "  compat=%-3s lines=%d  connect+first-tokens=%.2fs  edits: %s  mean-edit=%.2fs  ACCUM=%d\n"
                   (if (plist-get r :compat) "ON" "off")
                   (plist-get r :lines)
                   (plist-get r :connect-tokens)
                   (mapconcat (lambda (x) (format "%.2fs" x)) et " ")
                   (if et (/ (apply #'+ et) (float (length et))) 0.0)
                   (plist-get r :accum)))))

(cl-defun exp333-stress (compat nprocs)
  "Reproduce the reporter's stale-window trigger: rapid edit→undo bursts
that do NOT wait for the token response, so eglot repaints from stale
state while responses lag / are dropped.  Return accumulation count."
  (let* ((tmpdir (expand-file-name "tmp/eglot_test" t333-repo))
         (path (expand-file-name (format "stress-%s.tcl" (if compat "on" "off")) tmpdir))
         (src (exp333-big-tcl nprocs))
         (t333-eglot-compat compat))
    (make-directory tmpdir t)
    (with-temp-file path (insert src))
    (find-file path) (tcl-mode) (font-lock-mode 1)
    (let ((c (eglot--guess-contact)))
      (eglot--connect (nth 0 c) (nth 1 c) (nth 2 c) (nth 3 c) (nth 4 c)))
    (exp333-wait-fresh-tokens 60)   ; clean baseline (sets eglot--semtok-faces)
    ;; Rapid edit→undo bursts near the middle, repainting WITHOUT fully
    ;; draining the server response each time — this keeps eglot on its stale
    ;; `eglot--semtok-font-lock-2' repaint path across many font-lock passes.
    (goto-char (point-min))
    (search-forward (format "step%d = " (/ nprocs 2)) nil t)
    (let ((p (match-beginning 0)))
      (dotimes (_ 30)
        (goto-char p) (insert "Z")
        (eglot--signal-textDocument/didChange)
        (font-lock-flush) (font-lock-ensure)      ; stale repaint (no drain)
        (accept-process-output nil 0.005)
        (goto-char p) (delete-char 1)             ; undo
        (eglot--signal-textDocument/didChange)
        (font-lock-flush) (font-lock-ensure)      ; stale repaint (no drain)
        (accept-process-output nil 0.005)))
    (font-lock-flush) (font-lock-ensure)
    (let ((accum (exp333-accum-count)))
      (ignore-errors (eglot-shutdown (eglot-current-server) 1 nil))
      (let ((kill-buffer-query-functions nil)) (kill-buffer (current-buffer)))
      accum)))

(let ((nprocs (string-to-number (or (getenv "EXP_NPROCS") "600"))))  ; ~5400 lines
  (princ (format "\n===== issue #333 differential + perf (nprocs=%d) =====\n" nprocs))
  (let ((off (exp333-run nil nprocs))
        (on  (exp333-run t   nprocs)))
    (exp333-report off)
    (exp333-report on)
    (princ "\n  -- stale-window stress (rapid edit/undo, no drain) --\n")
    (let ((soff (exp333-stress nil nprocs))
          (son  (exp333-stress t   nprocs)))
      (princ (format "  stress accum: OFF=%d  ON=%d\n" soff son)))
    (princ (format "\n  RESULT: accum OFF=%d  ON=%d  →  %s\n"
                   (plist-get off :accum) (plist-get on :accum)
                   (cond ((and (> (plist-get off :accum) 0)
                               (= (plist-get on :accum) 0))
                          "PROVEN: compat ON eliminates accumulation, off reproduces it")
                         ((= (plist-get off :accum) (plist-get on :accum))
                          "INCONCLUSIVE: accumulation same in both (batch paint may not reproduce)")
                         (t "PARTIAL: accumulation differs but not cleanly")))))
  (kill-emacs 0))
