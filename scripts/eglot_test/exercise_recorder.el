;; -*- lexical-binding: t -*-
;;; Smoke-test for tcl-lsp-record-bug.el.
;;
;; Loads the recorder, drives a short eglot session against `uv run python -m lsp`,
;; calls every public command, and prints a summary of the resulting .eld file.

(require 'eglot)
(require 'tcl)
(require 'cl-lib)

(setq inhibit-message t
      message-log-max nil
      eglot-sync-connect 10
      eglot-events-buffer-config '(:size 2000000 :format full))

(load (expand-file-name "scripts/eglot_test/tcl-lsp-record-bug.el"
                        (or (getenv "TCL_LSP_REPO") default-directory)))

(setq tcl-lsp-bug-record-include-source t)

(setq eglot-server-programs
      `((tcl-mode . ("uv" "run" "--directory"
                     ,(or (getenv "TCL_LSP_REPO") default-directory)
                     "--no-dev" "python" "-m" "lsp"))))

(let* ((repo (or (getenv "TCL_LSP_REPO") default-directory))
       (dir (expand-file-name "tmp/eglot_test" repo))
       (path (expand-file-name "recorder-smoke.tcl" dir))
       (out (expand-file-name "recorder-smoke.eld" dir)))
  (make-directory dir t)
  (with-temp-file path
    (insert "namespace eval ::demo {}\n"
            "set x 1\n"
            "proc add {a b} { return [expr {$a + $b}] }\n"))

  (find-file path)
  (tcl-mode)
  (font-lock-mode 1)
  (let* ((c (eglot--guess-contact)))
    (eglot--connect (nth 0 c) (nth 1 c) (nth 2 c) (nth 3 c) (nth 4 c)))
  ;; Wait for initial analysis.
  (let ((deadline (+ (float-time) 6)))
    (while (< (float-time) deadline)
      (accept-process-output nil 0.05)
      (sit-for 0.02)))
  (font-lock-flush) (font-lock-ensure)

  (tcl-lsp-bug-record-start)
  ;; Drive a couple of edits.
  (goto-char (point-min))
  (insert "# bug demo\n")
  (let ((deadline (+ (float-time) 1.5)))
    (while (< (float-time) deadline)
      (accept-process-output nil 0.05) (sit-for 0.02)))
  (search-forward "set x 1") (replace-match "set x 42")
  (let ((deadline (+ (float-time) 1.5)))
    (while (< (float-time) deadline)
      (accept-process-output nil 0.05) (sit-for 0.02)))
  (font-lock-flush) (font-lock-ensure)

  (tcl-lsp-bug-record-mark "post-rename — does this look wrong?")

  (search-forward "$a + $b") (replace-match "$a * $b")
  (let ((deadline (+ (float-time) 1.5)))
    (while (< (float-time) deadline)
      (accept-process-output nil 0.05) (sit-for 0.02)))
  (font-lock-flush) (font-lock-ensure)
  (tcl-lsp-bug-record-mark "post-arith-edit")

  (tcl-lsp-bug-record-stop)
  (tcl-lsp-bug-record-save out)

  (princ (format "wrote: %s (%d bytes)\n"
                 out (nth 7 (file-attributes out))))
  (princ "\nrecording summary:\n")
  (with-temp-buffer
    (insert-file-contents out)
    ;; Skip header comments to the data sexp.
    (goto-char (point-min))
    (while (looking-at ";;") (forward-line 1))
    (let* ((events (read (current-buffer)))
           (kinds (cl-loop for e in events
                           for k = (car-safe (cdr-safe e))
                           collect k)))
      (princ (format "  total events: %d\n" (length events)))
      (dolist (k (cl-remove-duplicates kinds :test #'eq))
        (princ (format "    %-16s  ×%d\n" k (cl-count k kinds))))
      (princ "\n  :start versions block:\n")
      (let* ((start (cl-find-if (lambda (e)
                                  (eq (cadr e) :start))
                                events))
             (vers (plist-get (cdr start) :versions)))
        (cl-loop for (k v) on vers by #'cddr
                 do (princ (format "    %-22s %S\n" k v))))))
  (kill-emacs 0))
