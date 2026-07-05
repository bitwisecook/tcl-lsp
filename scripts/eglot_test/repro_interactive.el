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

;;; repro_interactive.el — issue #333 under a REAL redisplay (run via xvfb).
;;
;; Batch Emacs never runs jit-lock/redisplay, so `eglot--semtok-font-lock-2`'s
;; stale additive repaint never fires and the face accumulation can't be
;; reproduced there.  A real GUI frame drives jit-lock during `redisplay', so
;; that is where issue #333 actually appears.  We run the SAME rapid edit/undo
;; burst with eglot-compat OFF and ON and count accumulated `eglot-semantic-*`
;; faces after real redisplay, writing a RESULT line that self-classifies:
;; PROVEN / PARTIAL / NO-DIFFERENCE / NOT-REPRODUCED.
;;
;; IMPORTANT: this needs a *real, reasonably fast* GUI Emacs.  A software
;; `xvfb-run' frame renders each `redisplay' so slowly that even a small file
;; times out — run it on your own graphical Emacs rather than in CI:
;;
;;   TCL_LSP_REPO=$PWD TCL_LSP_SERVER_BIN=$PWD/target/release/tcl-lsp-server \
;;   TCL_LSP_T333_NOEXEC=1 RI_OUT=/tmp/ri.txt emacs -Q \
;;     -L tmp/elpa/eglot-1.24 -L tmp/elpa/jsonrpc-1.0.29 \
;;     -l scripts/eglot_test/test_issue333.el \
;;     -l scripts/eglot_test/repro_interactive.el
;;   # then read the result:  cat /tmp/ri.txt

(defun ri-big-tcl (n)
  (let ((s (list "namespace eval ::bench {\n    variable counter 0\n}\n\n")))
    (dotimes (i n)
      (push (format (concat "# proc %d\nproc ::bench::step%d {a b} {\n"
                            "    set v%d [expr {$a + $b}]\n"
                            "    set msg \"step %d = $v%d\"\n"
                            "    if {$v%d > 10} {\n        set v%d [expr {$v%d + 1}]\n    }\n"
                            "    return $v%d\n}\n\n")
                    i i i i i i i i i)
            s))
    (apply #'concat (nreverse s))))

(defun ri-worst-stack (snap)
  "Deepest eglot-semantic-* face stack on any single position in SNAP."
  (cl-loop for cell in snap
           for eg = (cl-remove-if-not
                     (lambda (f) (and (symbolp f)
                                      (string-prefix-p "eglot-semantic-" (symbol-name f))))
                     (t333-face-list (caddr cell)))
           maximize (length eg)))

(defun ri-snapshot ()
  (cl-loop for pos from (point-min) below (point-max)
           collect (list pos (char-after pos) (get-text-property pos 'face))))

(cl-defun ri-run (compat nprocs)
  "Reproduce issue #333 under real redisplay.  The trigger (from the
reporter): edit → repaint while the token response is still in flight →
undo, repeated.  We repaint with `redisplay' but do NOT drain the eglot
process between edits, so `eglot--docver' keeps moving ahead of the
server's answers and eglot stays on its stale `eglot--semtok-font-lock-2'
additive-repaint path.  Returns accumulation at the burst peak and after
a final settle."
  (let* ((tmpdir (expand-file-name "tmp/eglot_test" t333-repo))
         (realfile (getenv "RI_FILE"))
         (path (expand-file-name (format "ri-%s.tcl" (if compat "on" "off")) tmpdir))
         (t333-eglot-compat compat))
    (make-directory tmpdir t)
    (if (and realfile (file-exists-p realfile))
        (copy-file realfile path t)
      (with-temp-file path (insert (ri-big-tcl nprocs))))
    (ignore-errors (set-frame-size (selected-frame) 80 40))
    (find-file path) (tcl-mode) (font-lock-mode 1)
    (let ((c (eglot--guess-contact)))
      (eglot--connect (nth 0 c) (nth 1 c) (nth 2 c) (nth 3 c) (nth 4 c)))
    (ignore-errors (eglot-semantic-tokens-mode 1))
    ;; Baseline: settle real tokens on the region we'll edit (this sets the
    ;; `eglot--semtok-faces' props the stale painter later re-adds).
    (goto-char (point-min))
    (forward-line (/ (count-lines (point-min) (point-max)) 2))
    (recenter)
    (let ((deadline (+ (float-time) 20)))
      (while (and (< (float-time) deadline)
                  (or (null (cl-getf eglot--semtok-state :data))
                      (not (eq (cl-getf eglot--semtok-state :docver) eglot--docver))))
        (font-lock-flush) (font-lock-ensure) (accept-process-output nil 0.05) (sit-for 0.02)))
    (redisplay t)
    ;; Stale-window burst: edit/undo at a visible position, repainting via
    ;; `redisplay' WITHOUT draining the server between steps.
    (let ((p (point)))
      (dotimes (_ (string-to-number (or (getenv "RI_ITERS") "30")))
        (goto-char p) (insert "Z")
        (eglot--signal-textDocument/didChange)
        (redisplay t)                 ; jit-lock paints visible region from stale state
        (goto-char p) (delete-char 1) ; undo
        (eglot--signal-textDocument/didChange)
        (redisplay t)))
    (redisplay t)
    (let ((peak-snap (ri-snapshot)))
      ;; Now fully settle (drain + clean repaint) and re-measure: does the
      ;; accumulation clear on its own, or persist like the reporter sees?
      (let ((deadline (+ (float-time) 15)))
        (while (and (< (float-time) deadline)
                    (not (eq (cl-getf eglot--semtok-state :docver) eglot--docver)))
          (font-lock-flush) (accept-process-output nil 0.05) (sit-for 0.02) (redisplay t)))
      (redisplay t)
      (let* ((settled-snap (ri-snapshot))
             (res (list :compat compat
                        :peak-accum (length (t333-find-accumulated-faces peak-snap))
                        :peak-worst (ri-worst-stack peak-snap)
                        :settled-accum (length (t333-find-accumulated-faces settled-snap))
                        :settled-worst (ri-worst-stack settled-snap))))
        (ignore-errors (eglot-shutdown (eglot-current-server) 1 nil))
        (let ((kill-buffer-query-functions nil)) (kill-buffer (current-buffer)))
        res))))

;; In a GUI (non-batch) Emacs `princ' writes to the echo area, not stdout, so
;; results are written to a file the caller reads.
(defun ri-emit (s)
  (let ((f (or (getenv "RI_OUT")
               (expand-file-name "tmp/eglot_test/ri-result.txt" t333-repo))))
    (write-region s nil f 'append)))

(let ((n (string-to-number (or (getenv "RI_NPROCS") "600"))))
  (ri-emit (format "\n===== issue #333 interactive repro (real redisplay) =====\n"))
  (ri-emit (format "  graphic=%s  file=%s\n" (display-graphic-p)
                   (or (getenv "RI_FILE") (format "synthetic(nprocs=%d)" n))))
  (ri-emit "  (peak = right after the edit/undo burst; settled = after draining + clean repaint)\n")
  (let* ((off (with-timeout (150 (ri-emit "  (off run hit 150s timeout)\n") '(:peak-accum -1 :peak-worst -1 :settled-accum -1))
                (prog1 (ri-run nil n) (ri-emit "  off run done\n"))))
         (on  (with-timeout (150 (ri-emit "  (on run hit 150s timeout)\n") '(:peak-accum -1 :peak-worst -1 :settled-accum -1))
                (prog1 (ri-run t n) (ri-emit "  on run done\n"))))
         (op (plist-get off :peak-accum)) (opw (plist-get off :peak-worst))
         (os (plist-get off :settled-accum))
         (np (plist-get on :peak-accum)) (npw (plist-get on :peak-worst))
         (ns (plist-get on :settled-accum)))
    (ri-emit (format "  compat=off  peak: pos=%d worst-stack=%d   settled: pos=%d\n" op opw os))
    (ri-emit (format "  compat=ON   peak: pos=%d worst-stack=%d   settled: pos=%d\n" np npw ns))
    (ri-emit (format "\n  RESULT: %s\n"
                     (cond ((and (> op 0) (= np 0))
                            "PROVEN — compat ON eliminates the accumulation that OFF exhibits")
                           ((and (> op 0) (< np op))
                            (format "PARTIAL — compat ON reduces accumulation (%d→%d peak)" op np))
                           ((= op 0)
                            "NOT-REPRODUCED — even OFF shows no accumulation in this run")
                           (t "NO-DIFFERENCE — accumulation the same ON and OFF")))))
  (kill-emacs 0))
