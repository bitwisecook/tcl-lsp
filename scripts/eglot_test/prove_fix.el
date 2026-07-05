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

;;; prove_fix.el — prove which fix actually stops the issue #333 accumulation.
;;
;; Reproduces the painter accumulation (one clean paint + N stale repaints) and
;; measures the worst per-character `eglot-semantic-*' face stack under three
;; configurations:
;;
;;   1. baseline            — stock eglot, our server (compat OFF)
;;   2. server compat mode  — stock eglot, our server (compat ON)  [inert]
;;   3. client painter fix  — eglot advised to strip stale `eglot-semantic-*'
;;                            faces before each (re)paint, our server
;;
;; A fix "works" when the worst stack collapses to 1 (each char wears exactly
;; one semantic face).  This isolates whether the fix belongs on the server
;; (config mode) or the client (painter).
;;
;;   TCL_LSP_REPO=$PWD TCL_LSP_SERVER_BIN=$PWD/target/release/tcl-lsp-server \
;;   TCL_LSP_T333_NOEXEC=1 RI_FILE=/path/to/real.tcl \
;;   emacs -Q -batch -L tmp/elpa/eglot-1.24 -L tmp/elpa/jsonrpc-1.0.29 \
;;     -l scripts/eglot_test/test_issue333.el \
;;     -l scripts/eglot_test/prove_fix.el

;;; --- the candidate client-side fix (generalises georgtree's workaround) ---
;; Strip any `eglot-semantic-*' faces from BEG..END before eglot repaints, so
;; `add-face-text-property' can't stack a new copy on top of the old ones.

(defun pf--semantic-face-p (f)
  (and (symbolp f) (string-prefix-p "eglot-semantic-" (symbol-name f))))

(defun pf--strip (face)
  (cond ((null face) nil)
        ((pf--semantic-face-p face) nil)
        ((symbolp face) face)
        ((and (consp face) (keywordp (car face))) face)
        ((consp face)
         (let ((new (delq nil (mapcar #'pf--strip face))))
           (cond ((null new) nil) ((null (cdr new)) (car new)) (t new))))
        (t face)))

(defun pf--clear (beg end &rest _)
  (with-silent-modifications
    (let ((pos beg))
      (while (< pos end)
        (let* ((next (or (next-single-property-change pos 'face nil end) end))
               (new (pf--strip (get-text-property pos 'face))))
          (if new (put-text-property pos next 'face new)
            (remove-text-properties pos next '(face nil)))
          (setq pos next))))))

(defun pf-install-fix ()
  (advice-add 'eglot--semtok-font-lock-1 :before #'pf--clear)
  (advice-add 'eglot--semtok-font-lock-2 :before #'pf--clear))
(defun pf-remove-fix ()
  (advice-remove 'eglot--semtok-font-lock-1 #'pf--clear)
  (advice-remove 'eglot--semtok-font-lock-2 #'pf--clear))

;;; --- reproduction ---------------------------------------------------------

(defun pf-worst ()
  (cl-loop for pos from (point-min) below (point-max)
           maximize (length (cl-remove-if-not
                             #'pf--semantic-face-p
                             (t333-face-list (get-text-property pos 'face))))))

(defun pf-worst-dup ()
  "Deepest count of a SINGLE repeated eglot-semantic-* face on any char
 (>1 means true accumulation; 1 means every face is distinct/legitimate)."
  (cl-loop for pos from (point-min) below (point-max)
           for sem = (cl-remove-if-not #'pf--semantic-face-p
                                       (t333-face-list (get-text-property pos 'face)))
           maximize (or (cl-loop for f in (cl-remove-duplicates sem :test #'eq)
                                 maximize (cl-count f sem :test #'eq))
                        0)))

(cl-defun pf-run (label compat fix &key (repaints 6))
  (let* ((tmpdir (expand-file-name "tmp/eglot_test" t333-repo))
         (realfile (getenv "RI_FILE"))
         (path (expand-file-name "pf.tcl" tmpdir))
         (t333-eglot-compat compat))
    (make-directory tmpdir t)
    (if (and realfile (file-exists-p realfile)) (copy-file realfile path t)
      (with-temp-file path
        (dotimes (i 200)
          (insert (format "proc ::ns::s%d {a b} {\n    set v%d [expr {$a+$b}]\n    return \"$v%d ok\"\n}\n" i i i)))))
    (when fix (pf-install-fix))
    (unwind-protect
        (progn
          (t333-connect-and-open path)
          (t333-pump-until-semtok 15)
          (let ((data (cl-getf eglot--semtok-state :data)))
            (with-silent-modifications
              (remove-text-properties (point-min) (point-max)
                                      '(face nil eglot--semtok-faces nil eglot--semtok-token nil)))
            (eglot--semtok-font-lock-1 (point-min) (point-max) data)
            (dotimes (_ repaints) (eglot--semtok-font-lock-2 (point-min) (point-max)))
            (let ((worst (pf-worst)) (dup (pf-worst-dup)) (ntok (/ (length data) 5)))
              (t333-disconnect)
              (princ (format "  %-34s worst-stack=%2d  worst-DUPLICATE=%d  (ntok=%d)\n" label worst dup ntok))
              worst)))
      (when fix (pf-remove-fix)))))

(princ "\n===== issue #333: which fix actually collapses the face stack? =====\n")
(princ (format "  file=%s  (worst-stack=1 means fixed; >1 means accumulating)\n\n"
               (or (getenv "RI_FILE") "synthetic")))
(let ((b (pf-run "1. baseline (server compat OFF)" nil nil))
      (m (pf-run "2. server eglot-compat mode ON" t   nil))
      (c (pf-run "3. client painter fix (compat OFF)" nil t)))
  (princ (format "\n  RESULT: baseline=%d  server-mode=%d  client-fix=%d\n" b m c))
  (princ (format "  → server mode %s ; client painter fix %s\n"
                 (if (< m b) "reduces accumulation" "does NOT help (inert)")
                 (if (<= c 1) "PROVABLY fixes it" (format "reduces to %d" c)))))
(kill-emacs 0)
