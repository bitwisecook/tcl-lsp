;; -*- lexical-binding: t -*-
;;; compare_langs.el — cross-language eglot semantic-token timing comparison.
;;
;; Drives eglot against several language servers on similar-sized (~6k line)
;; inputs and reports, per server, how semantic-token highlighting APPEARS
;; (connect → first tokens) and UPDATES (edit → fresh tokens).  The point is
;; to see whether tcl-lsp's behaviour is typical of eglot semantic tokens in
;; general or an outlier — issue #333 context.
;;
;; Servers probed (when present): tcl-lsp, pyright (Python), rust-analyzer.
;;
;;   TCL_LSP_REPO=$PWD TCL_LSP_SERVER_BIN=$PWD/target/release/tcl-lsp-server \
;;   emacs -Q -batch -L tmp/elpa/eglot-1.24 -L tmp/elpa/jsonrpc-1.0.29 \
;;     -l scripts/eglot_test/compare_langs.el

(require 'eglot)
(require 'cl-lib)
(require 'python)

(setq eglot-sync-connect 10 inhibit-message t message-log-max nil
      eglot-events-buffer-config '(:size 200000 :format full)
      eglot-autoshutdown t)
(advice-add 'display-warning :around
            (lambda (orig type msg &rest r)
              (unless (or (eq type 'jsonrpc) (and (listp type) (memq 'jsonrpc type)))
                (apply orig type msg r))))

(defvar cmp-repo (or (getenv "TCL_LSP_REPO") (expand-file-name default-directory)))
(defvar cmp-root (expand-file-name "tmp/eglot_test/compare" cmp-repo))

;; A throwaway major mode for Rust so eglot can associate a server (no
;; rust-mode is bundled with Emacs).
(define-derived-mode cmp-rust-mode prog-mode "CmpRust")

(defun cmp-now () (float-time))

(defun cmp-wait-tokens (max-secs)
  "Block until eglot has semantic-token data for the current docver.
Return seconds waited (MAX-SECS on timeout)."
  (let ((start (cmp-now)) (deadline (+ (cmp-now) max-secs)))
    (while (and (< (cmp-now) deadline)
                (or (null (cl-getf eglot--semtok-state :data))
                    (not (eq (cl-getf eglot--semtok-state :docver) eglot--docver))))
      (font-lock-flush) (font-lock-ensure)
      (accept-process-output nil 0.02) (sit-for 0.01))
    (font-lock-flush) (font-lock-ensure)
    (- (cmp-now) start)))

(defun cmp-tokens-len ()
  (let ((d (cl-getf eglot--semtok-state :data)))
    (if (vectorp d) (/ (length d) 5) 0)))

(cl-defun cmp-run (&key name file mode server lang-id edits (appear-budget 90))
  "Connect eglot (SERVER) on FILE in MODE, time appear + edits.
Returns a plist of results, or nil on failure."
  (condition-case err
      (progn
        (setq eglot-server-programs
              (list (cons (if lang-id (list mode :language-id lang-id) mode)
                          server)))
        (find-file file)
        (funcall mode)
        (font-lock-mode 1)
        (let* ((c (eglot--guess-contact))
               (t0 (cmp-now)))
          (eglot--connect (nth 0 c) (nth 1 c) (nth 2 c) (nth 3 c) (nth 4 c))
          (ignore-errors (eglot-semantic-tokens-mode 1))
          (let* ((appear (cmp-wait-tokens appear-budget))
                 (ntok (cmp-tokens-len))
                 (lines (count-lines (point-min) (point-max)))
                 (edit-times '()))
            (dolist (e edits)
              (goto-char (point-min))
              (when (search-forward (car e) nil t)
                (replace-match (cdr e) t t)
                (eglot--signal-textDocument/didChange)
                (push (cmp-wait-tokens 60) edit-times)))
            (prog1
                (list :name name :lines lines :ntok ntok
                      :appear appear :edit-times (nreverse edit-times))
              (ignore-errors (eglot-shutdown (eglot-current-server) 1 nil))
              (let ((kill-buffer-query-functions nil)) (kill-buffer (current-buffer)))))))
    (error
     (princ (format "  %-14s ERROR: %S\n" name err))
     nil)))

(defun cmp-report (r)
  (when r
    (let ((et (plist-get r :edit-times)))
      (princ (format "  %-14s lines=%-5d tokens=%-6d  appear=%5.2fs  edits=[%s]  mean-update=%.2fs\n"
                     (plist-get r :name) (plist-get r :lines) (plist-get r :ntok)
                     (plist-get r :appear)
                     (mapconcat (lambda (x) (format "%.2f" x)) et " ")
                     (if et (/ (apply #'+ et) (float (length et))) 0.0))))))

;;; ---- generators (~6k lines each) --------------------------------------

(defun cmp-gen-tcl (n path)
  (with-temp-file path
    (insert "namespace eval ::bench {\n    variable counter 0\n}\n\n")
    (dotimes (i n)
      (insert (format (concat "# proc %d\nproc ::bench::step%d {a b} {\n"
                              "    set v%d [expr {$a + $b}]\n"
                              "    set msg \"step %d = $v%d\"\n"
                              "    if {$v%d > 10} { set v%d [expr {$v%d + 1}] }\n"
                              "    return $v%d\n}\n\n")
                      i i i i i i i i i)))))

(defun cmp-gen-py (n path)
  (with-temp-file path
    (insert "import math\n\n\n")
    (dotimes (i n)
      (insert (format (concat "# func %d\ndef step%d(a: int, b: int) -> int:\n"
                              "    v%d = a + b\n"
                              "    msg = f\"step %d = {v%d}\"\n"
                              "    if v%d > 10:\n        v%d = v%d + 1\n"
                              "    return v%d\n\n\n")
                      i i i i i i i i i)))))

(defun cmp-gen-rust (n dir)
  (make-directory (expand-file-name "src" dir) t)
  (with-temp-file (expand-file-name "Cargo.toml" dir)
    (insert "[package]\nname = \"bench\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n"))
  (let ((path (expand-file-name "src/lib.rs" dir)))
    (with-temp-file path
      (insert "//! bench\n\n")
      (dotimes (i n)
        (insert (format (concat "/// step %d\npub fn step%d(a: i64, b: i64) -> i64 {\n"
                                "    let v%d = a + b;\n"
                                "    let _msg = format!(\"step %d = {}\", v%d);\n"
                                "    if v%d > 10 { return v%d + 1; }\n"
                                "    v%d\n}\n\n")
                        i i i i i i i i))))
    path))

;;; ---- main -------------------------------------------------------------

(make-directory cmp-root t)
(princ "\n===== eglot semantic-token timing across languages (~6k lines) =====\n")

;; Tcl (tcl-lsp)
(let ((bin (or (getenv "TCL_LSP_SERVER_BIN")
               (expand-file-name "target/release/tcl-lsp-server" cmp-repo)))
      (f (expand-file-name "bench.tcl" cmp-root)))
  (cmp-gen-tcl 600 f)
  (when (file-exists-p bin)
    (cmp-report
     (cmp-run :name "tcl-lsp" :file f :mode 'tcl-mode :server (list bin)
              :edits '(("set v300 " . "set vv300 ") ("set v10 " . "set vv10 "))))))

;; Python (pyright)
(let ((f (expand-file-name "bench.py" cmp-root)))
  (cmp-gen-py 600 f)
  (when (executable-find "pyright-langserver")
    (cmp-report
     (cmp-run :name "pyright" :file f :mode 'python-mode
              :server (list "pyright-langserver" "--stdio") :lang-id "python"
              :edits '(("v300 = a + b" . "vv300 = a + b") ("v10 = a + b" . "vv10 = a + b"))))))

;; Rust (rust-analyzer)
(let* ((ra (string-trim (shell-command-to-string "rustup which rust-analyzer 2>/dev/null")))
       (dir (expand-file-name "rustbench" cmp-root))
       (f (cmp-gen-rust 600 dir)))
  (when (and ra (file-exists-p ra))
    (cmp-report
     (cmp-run :name "rust-analyzer" :file f :mode 'cmp-rust-mode
              :server (list ra) :lang-id "rust" :appear-budget 120
              :edits '(("let v300 = a" . "let vv300 = a") ("let v10 = a" . "let vv10 = a"))))))

(princ "\n  (appear = connect→first semantic tokens; update = edit→fresh tokens)\n")
(kill-emacs 0)
