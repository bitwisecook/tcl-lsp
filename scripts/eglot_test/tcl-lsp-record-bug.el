;;; tcl-lsp-record-bug.el --- Record edit + eglot traffic to debug tcl-lsp issues -*- lexical-binding: t -*-
;;
;; A self-contained emacs recorder for users hitting eglot+tcl-lsp issues
;; (e.g. https://github.com/bitwisecook/tcl-lsp/issues/333).
;;
;; Usage:
;;   1. Save this file somewhere, e.g. ~/.emacs.d/lisp/tcl-lsp-record-bug.el
;;   2. M-x load-file RET ~/.emacs.d/lisp/tcl-lsp-record-bug.el RET
;;   3. Open the Tcl file you can reproduce the issue in (with eglot running).
;;   4. M-x tcl-lsp-bug-record-start
;;   5. Reproduce the highlighting glitch:
;;        - make whatever edits trigger the bad highlighting
;;        - try saving the file, etc.
;;   6. M-x tcl-lsp-bug-record-stop
;;   7. M-x tcl-lsp-bug-record-save     (or it will prompt automatically)
;;        → writes ~/tcl-lsp-bug-<timestamp>.eld
;;   8. Attach the .eld file to the GitHub issue.
;;
;; What it captures (no source from your file is included unless you opt in
;; via the `tcl-lsp-bug-record-include-source' option):
;;   • emacs version, eglot version, eglot capabilities for the buffer
;;   • every after-change event (range + length, *not* the inserted text
;;     unless you opt in)
;;   • every JSON-RPC request/response between emacs and the LSP server
;;     (truncated to a configurable size)
;;   • a snapshot of `face' text-properties at the moments you trigger the
;;     "looks wrong" annotations via M-x tcl-lsp-bug-record-mark
;;
;; The output is plain elisp (read with `read'), human-inspectable.

(require 'cl-lib)
(require 'package)
(require 'eglot nil t)
(require 'jsonrpc nil t)

(defgroup tcl-lsp-bug-record nil
  "Record edit + eglot traffic for tcl-lsp debugging."
  :group 'eglot)

(defcustom tcl-lsp-bug-record-include-source nil
  "If non-nil, include buffer text and inserted-text snippets in the recording.
Default is nil so private code is not leaked.  When you enable this, any
text you type during the recording, plus the buffer's contents at start
and at every `tcl-lsp-bug-record-mark', WILL be written to the output."
  :type 'boolean)

(defcustom tcl-lsp-bug-record-max-payload-bytes 65536
  "Maximum bytes from any one JSON-RPC payload to include in the recording."
  :type 'integer)

(defvar tcl-lsp-bug-record--buffer nil
  "The buffer being recorded (only one at a time).")
(defvar tcl-lsp-bug-record--events nil
  "Recorded events, newest first.  Reversed on save.")
(defvar tcl-lsp-bug-record--start-time nil)
(defvar tcl-lsp-bug-record--server nil)


;;; ---------- helpers ----------------------------------------------------

(defun tcl-lsp-bug-record--ts ()
  "Return seconds since recording start, as a float."
  (if tcl-lsp-bug-record--start-time
      (- (float-time) tcl-lsp-bug-record--start-time)
    0.0))

(defun tcl-lsp-bug-record--push (event)
  "Prepend EVENT to the event log."
  (push (cons (tcl-lsp-bug-record--ts) event) tcl-lsp-bug-record--events))

(defun tcl-lsp-bug-record--truncate (s)
  "Truncate a string to `tcl-lsp-bug-record-max-payload-bytes'."
  (when (stringp s)
    (let ((max tcl-lsp-bug-record-max-payload-bytes))
      (if (<= (length s) max) s
        (concat (substring s 0 max)
                (format "...[+%d bytes]" (- (length s) max)))))))

(defun tcl-lsp-bug-record--face-snapshot ()
  "Return a compact run-length encoding of `face' properties.
Each element is (POS . FACE-VALUE)."
  (let (runs (last-face 'undefined))
    (save-excursion
      (goto-char (point-min))
      (while (not (eobp))
        (let ((f (get-text-property (point) 'face)))
          (unless (equal f last-face)
            (push (cons (- (point) (point-min)) f) runs)
            (setq last-face f)))
        (let ((next (or (next-single-property-change (point) 'face nil (point-max))
                        (point-max))))
          (goto-char next))))
    (nreverse runs)))

(defun tcl-lsp-bug-record--package-version (pkg)
  "Return an installed package version string, or nil."
  (when (and (require 'package nil t)
             (boundp 'package-alist))
    (let ((entry (cadr (assq pkg package-alist))))
      (when entry
        (package-version-join (package-desc-version entry))))))

(defun tcl-lsp-bug-record--shell-line (cmd)
  "Run CMD via the shell, return first non-empty stdout line, or nil."
  (ignore-errors
    (with-temp-buffer
      (let ((default-directory (or (and (boundp 'default-directory)
                                        default-directory)
                                   "~/")))
        (call-process-shell-command cmd nil t nil))
      (goto-char (point-min))
      (while (and (not (eobp))
                  (looking-at-p "^[[:space:]]*$"))
        (forward-line 1))
      (when (not (eobp))
        (string-trim
         (buffer-substring-no-properties
          (line-beginning-position) (line-end-position)))))))

(defun tcl-lsp-bug-record--collect-versions ()
  "Gather version numbers and OS info that may matter for reproduction."
  (list
   :emacs-version emacs-version
   :emacs-build (when (boundp 'emacs-build-time) emacs-build-time)
   :emacs-features (mapconcat
                    (lambda (f) (format "%s" f))
                    (cl-remove-if-not
                     (lambda (f)
                       (memq f '(native-compile xwidgets json
                                 tree-sitter modules dbus gnutls)))
                     (with-no-warnings (and (boundp 'features) features)))
                    " ")
   :system-type system-type
   :system-configuration system-configuration
   :system-name (system-name)
   :uname (or (tcl-lsp-bug-record--shell-line "uname -a") "")
   :linux-release (or (tcl-lsp-bug-record--shell-line
                       "lsb_release -ds 2>/dev/null || cat /etc/os-release | head -2 | tr '\\n' ' '")
                      "")
   :locale (cons (or (getenv "LANG") "") (or (getenv "LC_ALL") ""))
   :tty-or-graphic (if (display-graphic-p) 'graphic 'tty)
   :emacs-invocation
   (list :type initial-window-system
         :batch noninteractive
         :daemon (and (fboundp 'daemonp) (daemonp)))
   :font-lock-support-mode font-lock-support-mode
   :package-versions
   (cl-loop for p in '(eglot jsonrpc eldoc flymake project xref lsp-mode
                       company corfu)
            for v = (tcl-lsp-bug-record--package-version p)
            when v collect (cons p v))
   :eglot-version (when (fboundp 'eglot--version)
                    (ignore-errors (eglot--version)))
   :eglot-file (locate-library "eglot")
   :eglot-server-programs-entry
   (cl-find-if (lambda (e)
                 (or (eq (car-safe e) major-mode)
                     (and (consp (car e)) (memq major-mode (car e)))))
               eglot-server-programs)
   :tcl-lsp-cmd
   (when tcl-lsp-bug-record--server
     (process-command (jsonrpc--process tcl-lsp-bug-record--server)))
   :tcl-lsp-server-info
   (when tcl-lsp-bug-record--server
     (or (and (fboundp 'eglot--serverInfo)
              (ignore-errors (eglot--serverInfo tcl-lsp-bug-record--server)))
         (and (slot-exists-p tcl-lsp-bug-record--server 'server-info)
              (slot-value tcl-lsp-bug-record--server 'server-info))))
   ;; tclsh / uv / python versions if reachable on PATH.
   :uv-version (or (tcl-lsp-bug-record--shell-line "uv --version 2>/dev/null") "")
   :python-version (or (tcl-lsp-bug-record--shell-line "python3 --version 2>/dev/null") "")
   :tclsh-version (or (tcl-lsp-bug-record--shell-line "echo 'puts [info patchlevel]; exit' | tclsh 2>/dev/null") "")))

(defun tcl-lsp-bug-record--sanitize (val)
  "Recursively replace non-readable objects in VAL with strings.
Markers become their integer position; processes/buffers/hash-tables
become a textual placeholder."
  (cond
   ((markerp val) (marker-position val))
   ((processp val) (format "<process %s>" (process-name val)))
   ((bufferp val) (format "<buffer %s>" (buffer-name val)))
   ((hash-table-p val)
    (let (entries)
      (maphash (lambda (k v)
                 (push (cons (tcl-lsp-bug-record--sanitize k)
                             (tcl-lsp-bug-record--sanitize v))
                       entries))
               val)
      (cons 'hash-table entries)))
   ((vectorp val)
    (apply #'vector
           (mapcar #'tcl-lsp-bug-record--sanitize (append val nil))))
   ((consp val)
    (cons (tcl-lsp-bug-record--sanitize (car val))
          (tcl-lsp-bug-record--sanitize (cdr val))))
   ((or (functionp val) (subrp val) (byte-code-function-p val))
    "<function>")
   (t val)))

(defun tcl-lsp-bug-record--semtok-state ()
  "Return a copy of eglot's semtok plist for the buffer, if any."
  (when (and (boundp 'eglot--semtok-state) eglot--semtok-state)
    (let ((s (tcl-lsp-bug-record--sanitize
              (cl-copy-list eglot--semtok-state))))
      ;; Truncate :data to a length when source-inclusion is off.
      (when (and (vectorp (plist-get s :data))
                 (not tcl-lsp-bug-record-include-source))
        (setq s (plist-put s :data
                           (format "<vector len=%d>"
                                   (length (plist-get s :data))))))
      s)))


;;; ---------- hooks ------------------------------------------------------

(defun tcl-lsp-bug-record--on-change (beg end old-len)
  "after-change-functions hook: record the change region."
  (when (eq (current-buffer) tcl-lsp-bug-record--buffer)
    (tcl-lsp-bug-record--push
     (list :change
           :beg beg :end end :old-len old-len
           :inserted (when tcl-lsp-bug-record-include-source
                       (buffer-substring-no-properties beg end))
           :buffer-modified-tick (buffer-modified-tick)
           :eglot-docver (and (boundp 'eglot--docver) eglot--docver)))))

(defun tcl-lsp-bug-record--around-rpc-request (orig conn method params &rest rest)
  "Tap outgoing JSON-RPC requests."
  (when (eq (and (boundp 'eglot--cached-server) eglot--cached-server)
            tcl-lsp-bug-record--server)
    (tcl-lsp-bug-record--push
     (list :rpc-request :method method
           :params (tcl-lsp-bug-record--truncate
                    (prin1-to-string params)))))
  (apply orig conn method params rest))

(defun tcl-lsp-bug-record--around-rpc-notification (orig conn method params &rest rest)
  "Tap outgoing JSON-RPC notifications."
  (when (eq (and (boundp 'eglot--cached-server) eglot--cached-server)
            tcl-lsp-bug-record--server)
    (tcl-lsp-bug-record--push
     (list :rpc-notify :method method
           :params (tcl-lsp-bug-record--truncate
                    (prin1-to-string params)))))
  (apply orig conn method params rest))

(defun tcl-lsp-bug-record--around-handle-message (orig connection message)
  "Tap incoming JSON-RPC messages from the server."
  (when (and tcl-lsp-bug-record--server
             (eq (jsonrpc--process tcl-lsp-bug-record--server)
                 (jsonrpc--process connection)))
    (tcl-lsp-bug-record--push
     (list :rpc-incoming
           :method (plist-get message :method)
           :id (plist-get message :id)
           :payload (tcl-lsp-bug-record--truncate
                     (prin1-to-string message)))))
  (funcall orig connection message))


;;; ---------- public commands -------------------------------------------

;;;###autoload
(defun tcl-lsp-bug-record-start ()
  "Start recording editing and eglot traffic for the current buffer.
Stop with `tcl-lsp-bug-record-stop' and save with
`tcl-lsp-bug-record-save'."
  (interactive)
  (when tcl-lsp-bug-record--buffer
    (user-error "Recording already in progress in buffer %S — stop it first"
                tcl-lsp-bug-record--buffer))
  (unless (eglot-current-server)
    (user-error "Buffer has no eglot server connected"))
  (setq tcl-lsp-bug-record--buffer (current-buffer)
        tcl-lsp-bug-record--start-time (float-time)
        tcl-lsp-bug-record--server (eglot-current-server)
        tcl-lsp-bug-record--events nil)
  ;; Initial snapshot.
  (tcl-lsp-bug-record--push
   (list :start
         :versions (tcl-lsp-bug-record--collect-versions)
         :buffer-name (buffer-name)
         :file-name (when buffer-file-name
                      (file-name-nondirectory buffer-file-name))
         :major-mode major-mode
         :font-lock-mode font-lock-mode
         :buffer-size (- (point-max) (point-min))
         :buffer-text (when tcl-lsp-bug-record-include-source (buffer-string))
         :face-snapshot (tcl-lsp-bug-record--face-snapshot)
         :semtok-state (tcl-lsp-bug-record--semtok-state)
         :server-capabilities
         (when tcl-lsp-bug-record--server
           (eglot--capabilities tcl-lsp-bug-record--server))))
  ;; Hook in.
  (add-hook 'after-change-functions #'tcl-lsp-bug-record--on-change nil t)
  (when (fboundp 'jsonrpc-async-request)
    (advice-add 'jsonrpc-async-request :around
                #'tcl-lsp-bug-record--around-rpc-request))
  (when (fboundp 'jsonrpc-notify)
    (advice-add 'jsonrpc-notify :around
                #'tcl-lsp-bug-record--around-rpc-notification))
  (when (fboundp 'jsonrpc-connection-receive)
    (advice-add 'jsonrpc-connection-receive :around
                #'tcl-lsp-bug-record--around-handle-message))
  (message "tcl-lsp-bug-record: started.  Reproduce the issue, then M-x tcl-lsp-bug-record-mark when wrong, then M-x tcl-lsp-bug-record-stop."))

;;;###autoload
(defun tcl-lsp-bug-record-mark (label)
  "Add a labelled snapshot of the current buffer's face state."
  (interactive "sLabel for this snapshot (e.g. \"highlighting wrong here\"): ")
  (unless tcl-lsp-bug-record--buffer
    (user-error "No recording in progress"))
  (with-current-buffer tcl-lsp-bug-record--buffer
    (tcl-lsp-bug-record--push
     (list :mark
           :label label
           :buffer-text (when tcl-lsp-bug-record-include-source
                          (buffer-string))
           :buffer-size (- (point-max) (point-min))
           :face-snapshot (tcl-lsp-bug-record--face-snapshot)
           :semtok-state (tcl-lsp-bug-record--semtok-state)
           :eglot-docver (and (boundp 'eglot--docver) eglot--docver))))
  (message "Marked snapshot: %s" label))

;;;###autoload
(defun tcl-lsp-bug-record-stop ()
  "Stop recording.  Use `tcl-lsp-bug-record-save' to write the data."
  (interactive)
  (unless tcl-lsp-bug-record--buffer
    (user-error "No recording in progress"))
  (with-current-buffer tcl-lsp-bug-record--buffer
    (remove-hook 'after-change-functions #'tcl-lsp-bug-record--on-change t))
  (when (fboundp 'jsonrpc-async-request)
    (advice-remove 'jsonrpc-async-request
                   #'tcl-lsp-bug-record--around-rpc-request))
  (when (fboundp 'jsonrpc-notify)
    (advice-remove 'jsonrpc-notify
                   #'tcl-lsp-bug-record--around-rpc-notification))
  (when (fboundp 'jsonrpc-connection-receive)
    (advice-remove 'jsonrpc-connection-receive
                   #'tcl-lsp-bug-record--around-handle-message))
  (tcl-lsp-bug-record--push (list :stop))
  (let ((n (length tcl-lsp-bug-record--events))
        (buf tcl-lsp-bug-record--buffer))
    (setq tcl-lsp-bug-record--buffer nil
          tcl-lsp-bug-record--server nil)
    (message "tcl-lsp-bug-record: stopped.  %d events captured for %s.  M-x tcl-lsp-bug-record-save to write."
             n (buffer-name buf))))

;;;###autoload
(defun tcl-lsp-bug-record-save (path)
  "Write captured events to PATH (default: ~/tcl-lsp-bug-<timestamp>.eld)."
  (interactive
   (list (read-file-name
          "Save bug recording to: "
          "~/" nil nil
          (format "tcl-lsp-bug-%s.eld"
                  (format-time-string "%Y%m%d-%H%M%S")))))
  (when tcl-lsp-bug-record--buffer
    (user-error "Recording is still active — stop it first with M-x tcl-lsp-bug-record-stop"))
  (unless tcl-lsp-bug-record--events
    (user-error "No events to save"))
  (let ((events (mapcar #'tcl-lsp-bug-record--sanitize
                        (nreverse tcl-lsp-bug-record--events))))
    (with-temp-file path
      (let ((print-length nil)
            (print-level nil)
            (print-escape-newlines t))
        (insert ";; tcl-lsp bug recording -*- lisp-data -*-\n")
        (insert (format ";; created: %s\n"
                        (format-time-string "%Y-%m-%dT%H:%M:%S%z")))
        (insert (format ";; include-source: %s\n"
                        tcl-lsp-bug-record-include-source))
        (insert (format ";; events: %d\n\n" (length events)))
        (prin1 events (current-buffer))
        (insert "\n")))
    (message "Wrote %d events to %s%s"
             (length events) path
             (if tcl-lsp-bug-record-include-source
                 " (includes source text)"
               ""))
    ;; Clear so a subsequent recording starts fresh.
    (setq tcl-lsp-bug-record--events nil)))

;;;###autoload
(defun tcl-lsp-bug-record-status ()
  "Show current recording status."
  (interactive)
  (if tcl-lsp-bug-record--buffer
      (message "Recording in %s — %d events so far (include-source=%s)"
               (buffer-name tcl-lsp-bug-record--buffer)
               (length tcl-lsp-bug-record--events)
               tcl-lsp-bug-record-include-source)
    (message "Not recording.  %d events buffered from previous session."
             (length tcl-lsp-bug-record--events))))

(provide 'tcl-lsp-record-bug)
;;; tcl-lsp-record-bug.el ends here
