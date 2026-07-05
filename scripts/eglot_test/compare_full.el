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

;;; compare_full.el — issue #333 across servers, sizes, and the compat mode.
;;
;; Two jobs, one batch run (no redisplay needed — the painter functions are
;; driven directly):
;;
;;   A. TIMING: for each server and a range of input sizes, how long until
;;      semantic tokens first APPEAR (connect → first tokens) and how long an
;;      edit takes to UPDATE them.
;;
;;   B. PAINTER BUG: for each server, whether eglot's semantic-token painter
;;      accumulates duplicate `eglot-semantic-*' faces under the reporter's
;;      stale-repaint trigger — one clean paint then N stale repaints,
;;      counting the worst per-character face stack.  This is issue #333; if it
;;      reproduces with servers other than tcl-lsp it is purely eglot's painter.
;;
;; Servers probed when present: tcl-lsp (compat off AND on), pyright, clangd,
;; rust-analyzer.
;;
;;   TCL_LSP_REPO=$PWD TCL_LSP_SERVER_BIN=$PWD/target/release/tcl-lsp-server \
;;   TCL_LSP_T333_NOEXEC=1 emacs -Q -batch \
;;     -L tmp/elpa/eglot-1.24 -L tmp/elpa/jsonrpc-1.0.29 \
;;     -l scripts/eglot_test/test_issue333.el \
;;     -l scripts/eglot_test/compare_full.el

(require 'python)
(define-derived-mode cf-rust-mode prog-mode "CfRust")
(define-derived-mode cf-c-mode prog-mode "CfC")

(defvar cf-repo (or (getenv "TCL_LSP_REPO") default-directory))
(defvar cf-root (expand-file-name "tmp/eglot_test/cf" cf-repo))
(defun cf-now () (float-time))

(defun cf-wait-tokens (max-secs)
  (let ((start (cf-now)) (deadline (+ (cf-now) max-secs)))
    (while (and (< (cf-now) deadline)
                (or (null (cl-getf eglot--semtok-state :data))
                    (not (eq (cl-getf eglot--semtok-state :docver) eglot--docver))))
      (font-lock-flush) (font-lock-ensure) (accept-process-output nil 0.03) (sit-for 0.01))
    (- (cf-now) start)))

(defun cf-ntok () (let ((d (cl-getf eglot--semtok-state :data)))
                    (if (vectorp d) (/ (length d) 5) 0)))

(defun cf-worst-stack ()
  (cl-loop for pos from (point-min) below (point-max)
           for eg = (cl-remove-if-not
                     (lambda (f) (and (symbolp f)
                                      (string-prefix-p "eglot-semantic-" (symbol-name f))))
                     (t333-face-list (get-text-property pos 'face)))
           maximize (length eg)))

(defun cf-painter-accum (repaints)
  "One clean paint then REPAINTS stale repaints on the current buffer;
return worst per-char eglot-semantic-* stack, or 'no-tokens."
  (let ((data (cl-getf eglot--semtok-state :data)))
    (if (and (vectorp data) (> (length data) 0))
        (progn
          (with-silent-modifications
            (remove-text-properties (point-min) (point-max)
                                    '(face nil eglot--semtok-faces nil eglot--semtok-token nil)))
          (eglot--semtok-font-lock-1 (point-min) (point-max) data)
          (dotimes (_ repaints) (eglot--semtok-font-lock-2 (point-min) (point-max)))
          (cf-worst-stack))
      'no-tokens)))

(cl-defun cf-probe (&key name mode server lang-id init-opts file edits
                         (appear-budget 90) (repaints 6))
  "Connect eglot to SERVER on FILE, return timing + painter accumulation."
  (condition-case err
      (progn
        (setq eglot-server-programs
              (list (cons (if lang-id (list mode :language-id lang-id) mode) server)))
        (when init-opts
          (cl-defmethod eglot-initialization-options ((_s eglot-lsp-server)) init-opts))
        (unless init-opts
          (cl-defmethod eglot-initialization-options ((_s eglot-lsp-server)) eglot--{}))
        (find-file file) (funcall mode) (font-lock-mode 1)
        (let ((c (eglot--guess-contact)) (t0 (cf-now)))
          (eglot--connect (nth 0 c) (nth 1 c) (nth 2 c) (nth 3 c) (nth 4 c))
          (ignore-errors (eglot-semantic-tokens-mode 1))
          (let* ((appear (cf-wait-tokens appear-budget))
                 (ntok (cf-ntok))
                 (lines (count-lines (point-min) (point-max)))
                 (updates '()))
            (dolist (e edits)
              (goto-char (point-min))
              (when (search-forward (car e) nil t)
                (replace-match (cdr e) t t)
                (eglot--signal-textDocument/didChange)
                (push (cf-wait-tokens 60) updates)))
            (let ((worst (if (> ntok 0) (cf-painter-accum repaints) 'no-tokens)))
              (prog1 (list :name name :lines lines :ntok ntok :appear appear
                           :updates (nreverse updates) :worst worst)
                (ignore-errors (eglot-shutdown (eglot-current-server) 1 nil))
                (let ((kill-buffer-query-functions nil)) (kill-buffer (current-buffer))))))))
    (error (list :name name :error (format "%S" err)))))

(defun cf-report (r)
  (if (plist-get r :error)
      (princ (format "  %-22s ERROR %s\n" (plist-get r :name) (plist-get r :error)))
    (let ((u (plist-get r :updates)))
      (princ (format "  %-22s lines=%-5d tok=%-6d appear=%6.2fs update=%5.2fs  worst-stack=%s\n"
                     (plist-get r :name) (plist-get r :lines) (plist-get r :ntok)
                     (plist-get r :appear)
                     (if u (/ (apply #'+ u) (float (length u))) 0.0)
                     (plist-get r :worst))))))

;;; ---- file generators, size-parameterised ----------------------------

(defun cf-gen-tcl (n path)
  (with-temp-file path
    (insert "namespace eval ::bench {\n    variable counter 0\n}\n\n")
    (dotimes (i n)
      (insert (format (concat "proc ::bench::step%d {a b} {\n    set v%d [expr {$a + $b}]\n"
                              "    set msg \"step %d = $v%d\"\n    if {$v%d > 10} { set v%d [expr {$v%d + 1}] }\n"
                              "    return $v%d\n}\n\n") i i i i i i i i)))))

(defun cf-gen-py (n path)
  (with-temp-file path
    (insert "import math\n\n\n")
    (dotimes (i n)
      (insert (format (concat "def step%d(a: int, b: int) -> int:\n    v%d = a + b\n"
                              "    msg = f\"step %d = {v%d}\"\n    if v%d > 10:\n        v%d = v%d + 1\n"
                              "    return v%d\n\n\n") i i i i i i i i)))))

(defun cf-gen-c (n path)
  (with-temp-file path
    (insert "#include <stdio.h>\n\n")
    (dotimes (i n)
      (insert (format (concat "int step%d(int a, int b) {\n    int v%d = a + b;\n"
                              "    if (v%d > 10) { v%d = v%d + 1; }\n    return v%d;\n}\n\n")
                      i i i i i i)))))

(defun cf-gen-rust (n dir)
  (make-directory (expand-file-name "src" dir) t)
  (with-temp-file (expand-file-name "Cargo.toml" dir)
    (insert "[package]\nname=\"b\"\nversion=\"0.1.0\"\nedition=\"2021\"\n\n[lib]\npath=\"src/lib.rs\"\n"))
  (let ((path (expand-file-name "src/lib.rs" dir)))
    (with-temp-file path
      (dotimes (i n)
        (insert (format (concat "pub fn step%d(a: i64, b: i64) -> i64 {\n    let v%d = a + b;\n"
                                "    if v%d > 10 { return v%d + 1; }\n    v%d\n}\n\n")
                        i i i i i))))
    path))

;;; ---- main -------------------------------------------------------------

(make-directory cf-root t)
(princ "\n===== issue #333: eglot semantic tokens across servers / sizes / compat =====\n")
(princ "  (appear = connect->first tokens; update = edit->fresh tokens;\n")
(princ "   worst-stack = deepest eglot-semantic-* face stack after 1 clean + 6 stale repaints — >1 means the #333 painter bug reproduces)\n\n")

(let* ((bin (or (getenv "TCL_LSP_SERVER_BIN")
                (expand-file-name "target/release/tcl-lsp-server" cf-repo)))
       (ra (string-trim (shell-command-to-string "rustup which rust-analyzer 2>/dev/null")))
       (sizes (list (cons "S" 60) (cons "M" 400) (cons "L" 1200)))
       (tcl-edits '(("set v30 " . "set vv30 ") ("set v3 " . "set vv3 ")))
       (py-edits '(("v30 = a + b" . "vv30 = a + b") ("v3 = a + b" . "vv3 = a + b")))
       (c-edits '(("int v30 = a" . "int vv30 = a") ("int v3 = a" . "int vv3 = a")))
       (rs-edits '(("let v30 = a" . "let vv30 = a") ("let v3 = a" . "let vv3 = a"))))
  (dolist (sz sizes)
    (let ((tag (car sz)) (n (cdr sz)))
      (princ (format "-- size %s (~%d items) --\n" tag n))
      ;; tcl-lsp compat OFF and ON
      (let ((f (expand-file-name (format "b-%s.tcl" tag) cf-root)))
        (cf-gen-tcl n f)
        (when (file-exists-p bin)
          (cf-report (cf-probe :name (format "tcl-lsp[%s] compat-off" tag) :mode 'tcl-mode
                               :server (list bin) :file f :edits tcl-edits))
          (cf-report (cf-probe :name (format "tcl-lsp[%s] compat-ON" tag) :mode 'tcl-mode
                               :server (list bin) :file f :edits tcl-edits
                               :init-opts '(:tclLsp (:compatibility (:eglot t)))))))
      ;; pyright
      (when (executable-find "pyright-langserver")
        (let ((f (expand-file-name (format "b-%s.py" tag) cf-root)))
          (cf-gen-py n f)
          (cf-report (cf-probe :name (format "pyright[%s]" tag) :mode 'python-mode
                               :server '("pyright-langserver" "--stdio") :lang-id "python"
                               :file f :edits py-edits :appear-budget 60))))
      ;; clangd (C)
      (when (executable-find "clangd")
        (let ((f (expand-file-name (format "b-%s.c" tag) cf-root)))
          (cf-gen-c n f)
          (cf-report (cf-probe :name (format "clangd[%s]" tag) :mode 'cf-c-mode
                               :server '("clangd" "--background-index=false") :lang-id "c"
                               :file f :edits c-edits :appear-budget 60))))
      ;; rust-analyzer
      (when (and ra (file-exists-p ra))
        (let* ((dir (expand-file-name (format "rust-%s" tag) cf-root))
               (f (cf-gen-rust n dir)))
          (cf-report (cf-probe :name (format "rust-analyzer[%s]" tag) :mode 'cf-rust-mode
                               :server (list ra) :lang-id "rust"
                               :file f :edits rs-edits :appear-budget 150))))
      (princ "\n"))))
(kill-emacs 0)
