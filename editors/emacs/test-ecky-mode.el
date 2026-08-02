;;; test-ecky-mode.el --- Batch checks for ecky-mode -*- lexical-binding: t -*-

(require 'cl-lib)

(let* ((test-dir (file-name-directory (or load-file-name buffer-file-name)))
       (editors-dir (expand-file-name ".." test-dir))
       (fixture (expand-file-name "fixtures/full-syntax.ecky" editors-dir)))
  (add-to-list 'load-path test-dir)
  (require 'ecky-mode)
  (find-file fixture)
  (ecky-mode)
  (font-lock-ensure)

  (cl-labels
      ((has-face-p
        (value expected)
        (if (listp value) (memq expected value) (eq value expected)))
       (assert-token-face
        (token expected &optional occurrence-offset)
        (goto-char (point-min))
        (unless (search-forward token nil t)
          (error "Fixture token missing: %s" token))
        (let* ((position (+ (match-beginning 0) (or occurrence-offset 0)))
               (actual (get-text-property position 'face)))
          (unless (has-face-p actual expected)
            (error "%s face: expected %s, got %S" token expected actual)))))
    (assert-token-face "; Ecky CAD" 'font-lock-comment-face 2)
    (assert-token-face "\"Radius\"" 'font-lock-string-face 1)
    (assert-token-face "#f" 'ecky-boolean-face)
    (assert-token-face ":label" 'ecky-keyword-prefix-face)
    (assert-token-face "20" 'ecky-number-face)
    (assert-token-face "(box" 'ecky-function-face 1)
    (assert-token-face "(let*" 'ecky-keyword-face 1)
    (assert-token-face "define-component mounting-bracket"
                       'font-lock-function-name-face
                       (length "define-component "))
    (assert-token-face "(part body" 'font-lock-function-name-face 6))

  (unless (eq (cdr (assoc "\\.ecky\\'" auto-mode-alist)) 'ecky-mode)
    (error "Missing .ecky auto-mode association"))
  (message "ecky-mode runtime font-lock: PASS"))

;;; test-ecky-mode.el ends here
