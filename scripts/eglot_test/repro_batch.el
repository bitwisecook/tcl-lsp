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

;;; repro_batch.el — deterministic, fast reproduction of issue #333's painter
;;; accumulation, driving eglot's real painter functions (no redisplay needed).
;;
;; The reporter's trigger is: edit → the token response lags → eglot repaints
;; the region from stale local state several times before the fresh tokens
;; land.  Those stale repaints go through `eglot--semtok-font-lock-2', which
;; re-applies faces from the `eglot--semtok-faces' text property with
;; `add-face-text-property' (additive) and never strips the prior
;; `eglot-semantic-*' faces first — so each stale repaint stacks another copy.
;;
;; This script reproduces exactly that, deterministically and in batch: it does
;; ONE clean paint (`eglot--semtok-font-lock-1', which seeds `eglot--semtok-faces')
;; then N stale repaints (`eglot--semtok-font-lock-2') and counts accumulated
;; faces — with the server-side eglot-compat mode OFF and ON.  Because the
;; painter reads token data the server sends identically in both modes, this
;; isolates whether the capability shape changes the client-side accumulation.
;;
;;   TCL_LSP_REPO=$PWD TCL_LSP_SERVER_BIN=$PWD/target/release/tcl-lsp-server \
;;   TCL_LSP_T333_NOEXEC=1 RI_FILE=/path/to/real.tcl \
;;   emacs -Q -batch -L tmp/elpa/eglot-1.24 -L tmp/elpa/jsonrpc-1.0.29 \
;;     -l scripts/eglot_test/test_issue333.el \
;;     -l scripts/eglot_test/repro_batch.el

(defun rb-worst (snap)
  (cl-loop for cell in snap
           for eg = (cl-remove-if-not
                     (lambda (f) (and (symbolp f)
                                      (string-prefix-p "eglot-semantic-" (symbol-name f))))
                     (t333-face-list (caddr cell)))
           maximize (length eg)))

(defun rb-snapshot ()
  (cl-loop for pos from (point-min) below (point-max)
           collect (list pos (char-after pos) (get-text-property pos 'face))))

(cl-defun rb-run (compat &key (repaints 6))
  "Seed one clean paint, then REPAINTS stale repaints; return accumulation."
  (let* ((tmpdir (expand-file-name "tmp/eglot_test" t333-repo))
         (realfile (getenv "RI_FILE"))
         (path (expand-file-name (format "rb-%s.tcl" (if compat "on" "off")) tmpdir))
         (t333-eglot-compat compat))
    (make-directory tmpdir t)
    (if (and realfile (file-exists-p realfile))
        (copy-file realfile path t)
      (with-temp-file path
        (dotimes (i 200)
          (insert (format (concat "proc ::ns::step%d {a b} {\n"
                                  "    set v%d [expr {$a + $b}]\n"
                                  "    set msg \"step %d = $v%d\"\n"
                                  "    return $v%d\n}\n") i i i i i)))))
    (t333-connect-and-open path)
    (t333-pump-until-semtok 15)
    (let ((data (cl-getf eglot--semtok-state :data)))
      (unless (and data (vectorp data) (> (length data) 0))
        (t333-disconnect)
        (cl-return-from rb-run (list :compat compat :error "no tokens")))
      ;; Start from a genuinely clean slate (font-lock already painted once
      ;; during the pump), then do ONE clean paint that seeds
      ;; `eglot--semtok-faces' across the buffer — baseline accumulation ~0.
      (with-silent-modifications
        (remove-text-properties (point-min) (point-max)
                                '(face nil eglot--semtok-faces nil eglot--semtok-token nil)))
      (eglot--semtok-font-lock-1 (point-min) (point-max) data)
      (let ((after-clean (t333-find-accumulated-faces (rb-snapshot))))
        ;; Now the stale-window repaints: eglot's `-2' path, called the way a
        ;; lagging response + font-lock refontification would call it.
        (dotimes (_ repaints)
          (eglot--semtok-font-lock-2 (point-min) (point-max)))
        (let* ((snap (rb-snapshot))
               (res (list :compat compat
                          :clean-accum (length after-clean)
                          :accum (length (t333-find-accumulated-faces snap))
                          :worst (rb-worst snap)
                          :ntok (/ (length data) 5))))
          (t333-disconnect)
          res)))))

(let ((repaints (string-to-number (or (getenv "RB_REPAINTS") "6"))))
  (princ "\n===== issue #333 painter reproduction (batch, deterministic) =====\n")
  (princ (format "  file=%s  repaints=%d\n"
                 (or (getenv "RI_FILE") "synthetic") repaints))
  (let ((off (rb-run nil :repaints repaints))
        (on  (rb-run t   :repaints repaints)))
    (princ (format "  compat=off  ntok=%s  after-1-clean-paint=%s  after-%d-stale-repaints: pos=%s worst-stack=%s\n"
                   (plist-get off :ntok) (plist-get off :clean-accum) repaints
                   (plist-get off :accum) (plist-get off :worst)))
    (princ (format "  compat=ON   ntok=%s  after-1-clean-paint=%s  after-%d-stale-repaints: pos=%s worst-stack=%s\n"
                   (plist-get on :ntok) (plist-get on :clean-accum) repaints
                   (plist-get on :accum) (plist-get on :worst)))
    (let ((oa (plist-get off :accum)) (na (plist-get on :accum)))
      (princ (format "\n  RESULT: %s\n"
                     (cond ((not (numberp oa)) "ERROR — no tokens")
                           ((and (> oa 0) (= na 0))
                            "PROVEN — compat ON eliminates the accumulation OFF exhibits")
                           ((and (> oa 0) (< na oa))
                            (format "PARTIAL — compat ON reduces accumulation (%d→%d)" oa na))
                           ((= oa 0) "NOT-REPRODUCED — no accumulation even OFF")
                           (t "NO-DIFFERENCE — accumulation identical ON and OFF (the capability shape does not touch eglot's painter)")))))
    (kill-emacs 0)))
