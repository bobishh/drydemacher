;;; ecky-mode.el --- Major mode for Ecky CAD DSL -*- lexical-binding: t -*-

;; Author: Ecky Project
;; Keywords: languages, cad

;;; Commentary:

;; A major mode for .ecky files, a Scheme-derived DSL for CAD modeling.
;; Provides syntax highlighting, indentation, and font-lock for all
;; canonical forms, helpers, and CAD operations.

;;; Code:

(require 'scheme)

(defgroup ecky nil
  "Major mode for editing Ecky CAD DSL files."
  :prefix "ecky-"
  :group 'languages)

(defcustom ecky-mode-hook nil
  "Hook run when entering Ecky mode."
  :type 'hook
  :group 'ecky)

;; Font lock faces
(defface ecky-keyword-face
  '((t :inherit font-lock-keyword-face :weight bold))
  "Face for Ecky keywords (model clauses, wrappers)."
  :group 'ecky)

(defface ecky-function-face
  '((t :inherit font-lock-function-name-face))
  "Face for Ecky built-in functions and CAD operations."
  :group 'ecky)

(defface ecky-boolean-face
  '((t :inherit font-lock-constant-face))
  "Face for Ecky boolean literals."
  :group 'ecky)

(defface ecky-number-face
  '((t :inherit font-lock-number-face))
  "Face for Ecky number literals."
  :group 'ecky)

(defface ecky-string-face
  '((t :inherit font-lock-string-face))
  "Face for Ecky string literals."
  :group 'ecky)

(defface ecky-comment-face
  '((t :inherit font-lock-comment-face))
  "Face for Ecky comments."
  :group 'ecky)

(defface ecky-keyword-prefix-face
  '((t :inherit font-lock-variable-name-face))
  "Face for colon keyword prefixes."
  :group 'ecky)

;; Language constants from ecky_language_surface.rs
(defconst ecky-model-clauses
  '("params" "verify" "part" "feature" "meta"
    "tag-vertex" "tag-face" "tag-edge" "tag-edges" "view" "analysis")
  "Model clauses in Ecky.")

(defconst ecky-model-wrappers
  '("begin" "let" "let*")
  "Model wrappers in Ecky.")

(defconst ecky-expression-forms
  '("define" "lambda" "let" "let*" "begin" "if" "quote" "list" "append" "reverse"
    "range" "map" "filter" "fold" "reduce" "zip" "enumerate"
    "linspace" "flat-map" "concat-map" "apply")
  "Expression forms in Ecky.")

(defconst ecky-numeric-helpers
  '("pi" "tau" "+" "-" "*" "/" "min" "max" "abs" "floor"
    "sin" "cos" "tan" "atan" "atan2" "deg" "rad"
    "deg->rad" "rad->deg" "clamp" "lerp" "invlerp" "remap" "smoothstep"
    "square" "cube"
    "hash01" "hash-signed" "noise2" "fbm2" "voronoi2" "cell-distance2")
  "Numeric helpers in Ecky.")

(defconst ecky-point-list-helpers
  '("vec2" "vec3" "jitter2" "jittered-grid" "polar-points" "organic-loop"
    "wave-loop" "superellipse-point" "voronoi-cells"
    "lorenz-points" "rossler-points" "logistic-bifurcation-points"
    "henon-points")
  "Point list helpers in Ecky.")

(defconst ecky-boolean-helpers
  '("not" "and" "or" "=" ">" ">=" "<" "<="
    "even?" "odd?" "zero?" "null?" "empty?" "list?")
  "Boolean helpers in Ecky.")

(defconst ecky-cad-ops-portable
  '("box" "sphere" "cylinder" "cone" "circle" "ring"
    "rectangle" "rounded-rect" "rounded-polygon" "polygon"
    "profile" "make-face" "text" "svg" "import-stl"
    "path" "polyline" "bezier-path" "bspline"
    "extrude" "revolve" "loft" "sweep" "helical-ridge"
    "thread" "tapped-hole" "rib" "groove" "torus" "ellipse"
    "regular-polygon" "trapezoid" "wedge" "slot-overall"
    "slot-center-to-center" "slot-center-point" "slot-arc"
    "shell" "offset" "offset-rounded" "fillet" "chamfer"
    "taper" "draft" "twist" "union" "fuse" "difference" "cut"
    "intersection" "common" "xor" "compound"
    "translate" "rotate" "scale" "mirror"
    "linear-array" "radial-array" "grid-array" "arc-array"
    "repeat" "repeat-union" "repeat-compound" "repeat-pick"
    "for-union" "for-compound"
    "plane" "location" "path-frame" "place" "clip-box" "clip-plane"
    "build" "shape" "result" "sampled-radial-loft")
  "Portable CAD operations in Ecky.")

(defconst ecky-cad-ops-ecky-rust-only
  '("mesh" "polyhedron" "protrude" "wall-pattern" "surface-trim" "mesh-anchor")
  "EckyRust-only CAD operations in Ecky.")

(defconst ecky-cad-ops-ecky-rust-direct-only
  '("hull" "voronoi-cell" "import-step")
  "EckyRust direct-OCCT-only CAD operations in Ecky.")

(defconst ecky-wall-pattern-modes
  '("ribs" "rings" "spiral" "diamond" "hammered"
    "fourier" "cellular" "fbm" "gyroid" "schwarz-p"
    "schwarz-d" "diamond-field" "neovius" "attractor-field")
  "Wall pattern modes in Ecky.")

(defconst ecky-structural-forms
  '("model" "define-component" "verify" "tag" "metric" "expect"
    "number" "toggle" "select" "image" "option" "feature" "hole")
  "Ecky forms exported outside the backend operation arrays.")

;; Combined symbol lists
(defconst ecky-keywords
  (append ecky-model-clauses ecky-model-wrappers)
  "All keywords in Ecky.")

(defconst ecky-builtin-functions
  (append ecky-expression-forms
          ecky-numeric-helpers
          ecky-point-list-helpers
          ecky-boolean-helpers
          ecky-cad-ops-portable
          ecky-cad-ops-ecky-rust-only
          ecky-cad-ops-ecky-rust-direct-only
          ecky-wall-pattern-modes
          ecky-structural-forms)
  "All built-in functions in Ecky.")

;; Syntax table
(defvar ecky-mode-syntax-table
  (let ((table (make-syntax-table)))
    ;; Parenthesis pairs
    (modify-syntax-entry ?\( "()" table)
    (modify-syntax-entry ?\) ")(" table)
    ;; Strings
    (modify-syntax-entry ?\" "\"" table)
    ;; Comments: semicolon to end of line
    (modify-syntax-entry ?\; "<" table)
    (modify-syntax-entry ?\n ">" table)
    ;; Symbol constituents
    (modify-syntax-entry ?- "w" table)
    (modify-syntax-entry ?+ "w" table)
    (modify-syntax-entry ?* "w" table)
    (modify-syntax-entry ?/ "w" table)
    (modify-syntax-entry ?< "w" table)
    (modify-syntax-entry ?> "w" table)
    (modify-syntax-entry ?= "w" table)
    (modify-syntax-entry ?? "w" table)
    (modify-syntax-entry ?! "w" table)
    (modify-syntax-entry ?: "_" table)  ; Colon for keyword prefixes
    table)
  "Syntax table for Ecky mode.")

;; Font lock patterns
(defun ecky-font-lock-keywords ()
  "Create font lock keywords for Ecky mode."
  (let ((head-end "\\(?:\\s-\\|[()]\\)")
        (symbol-name "[^][()\";[:space:]]+"))
    `(
      ;; Definition-like forms: operator and declared name get distinct faces.
      (,(concat "(\\s-*\\(define-component\\|define\\|part\\)\\s-+\\("
                symbol-name "\\)")
       (1 'ecky-keyword-face)
       (2 'font-lock-function-name-face))

      ;; Form heads. Explicit delimiter matching also handles names ending in
      ;; punctuation: `let*`, `even?`, `+`, and `>=`.
      (,(concat "(\\s-*\\(" (regexp-opt ecky-keywords) "\\)" head-end)
       (1 'ecky-keyword-face))
      (,(concat "(\\s-*\\(" (regexp-opt ecky-builtin-functions) "\\)" head-end)
       (1 'ecky-function-face))

      ;; Only token captures receive faces; leading delimiters remain plain.
      ("\\(?:\\`\\|[][()[:space:]]\\)\\(#\\(?:t\\|f\\)\\)\\_>"
       (1 'ecky-boolean-face))
      ("\\(?:\\`\\|[][()[:space:]]\\)\\([+-]?\\(?:[0-9]+\\(?:\\.[0-9]*\\)?\\|\\.[0-9]+\\)\\(?:[eE][+-]?[0-9]+\\)?\\)\\_>"
       (1 'ecky-number-face))

      ;; Colon keyword prefix (e.g. :label, :mode).
      (":[[:alpha:]][[:alnum:]-]*" . 'ecky-keyword-prefix-face))))

;; Indentation function (Scheme-derived)
(defun ecky-indent-line ()
  "Indent current line as Ecky code."
  (interactive)
  (let ((indent-col (ecky-calculate-indentation))
        (pos (- (point-max) (point))))
    (if (<= (point) (save-excursion (beginning-of-line) (point)))
        (indent-line-to indent-col)
      (save-excursion
        (indent-line-to indent-col))
      (if (> (- (point-max) pos) (point))
          (goto-char (- (point-max) pos))))))

(defun ecky-calculate-indentation ()
  "Calculate the indentation column for the current line."
  (let ((in-paren-pos (save-excursion
                        (ecky-find-containing-paren))))
    (if in-paren-pos
        (let* ((op-start (save-excursion
                          (goto-char in-paren-pos)
                          (forward-char 1)
                          (skip-chars-forward " \t\n")
                          (point)))
               (op-end (save-excursion
                        (goto-char op-start)
                        (skip-chars-forward "a-zA-Z0-9-?!=<>+*/:")
                        (point)))
               (op (buffer-substring-no-properties op-start op-end))
               (base-indent (+ (save-excursion
                               (goto-char in-paren-pos)
                               (current-column))
                              2)))
          ;; Special cases for different forms
          (cond
           ;; Model clauses: body at same column as opening paren
           ((member op ecky-model-clauses)
            (save-excursion (goto-char in-paren-pos) (current-column)))
           ;; define/lambda: body indented
           ((or (string= op "define") (string= op "lambda"))
            (+ base-indent 2))
           ;; let/let*: bindings aligned with opening, body indented
           ((or (string= op "let") (string= op "let*"))
            (if (save-excursion
                  (beginning-of-line)
                  (looking-at "\\s-*(\\s-*$"))
                ;; Empty binding list, body comes next
                (+ base-indent 2)
              (save-excursion (goto-char in-paren-pos) (current-column))))
           ;; Default: body indented by 2 spaces from opening paren
           (t base-indent)))
      0)))

(defun ecky-find-containing-paren ()
  "Find the position of the containing \='(\=' for the current line."
  (save-excursion
    (beginning-of-line)
    (let ((parse-state (syntax-ppss)))
      (if (nth 1 parse-state)
          (nth 1 parse-state)
        ;; Try to find a paren on previous lines
        (let (found)
          (while (and (not found)
                      (re-search-backward "(" (point-min) t))
            (when (not (save-excursion
                        (goto-char (match-beginning 0))
                        (syntax-ppss-context (syntax-ppss))))
              (setq found (match-beginning 0))))
          found)))))

;; Mode definition
(define-derived-mode ecky-mode scheme-mode "Ecky"
  "Major mode for editing Ecky CAD DSL files."
  :syntax-table ecky-mode-syntax-table
  (setq-local font-lock-defaults (list (ecky-font-lock-keywords)))
  (setq-local comment-start ";")
  (setq-local comment-end "")
  (setq-local comment-start-skip ";+ *")
  (setq-local parse-sexp-ignore-comments t)
  (setq-local lisp-indent-function #'scheme-indent-function)
  (when (fboundp 'electric-pair-local-mode)
    (electric-pair-local-mode 1)))

;; Simplified lisp-indent-function replacement
(defun ecky-lisp-indent-function (_indent-point state)
  "Ecky-specific lisp indent function."
  (let ((normal-indent (current-column)))
    (goto-char (1+ (nth 1 state)))
    (if (or (not (looking-at "[a-zA-Z-]"))
            (and (not (member (buffer-substring-no-properties
                              (point)
                              (progn (skip-chars-forward "a-zA-Z-")
                                     (point)))
                             '("define" "lambda" "let" "let*")))
                 (progn (forward-sexp 1)
                        (skip-chars-forward " \t")
                        (not (looking-at "\n")))))
        normal-indent
      (+ normal-indent 2))))

;; Auto-mode association
(add-to-list 'auto-mode-alist '("\\.ecky\\'" . ecky-mode))

;; Imenu support
(defun ecky-imenu-create-index ()
  "Create an imenu index for Ecky buffers."
  (let ((index-alist '())
        (definitions '()))
    (save-excursion
      (goto-char (point-min))
      (while (re-search-forward
              "(\\s-*\\(?:define\\(?:-component\\)?\\|part\\)\\s-+\\([^][()\";[:space:]]+\\)"
              nil t)
        (push (cons (match-string 1) (match-beginning 1)) definitions)))
    (if definitions
        (cons "Definitions" (nreverse definitions))
      index-alist)))

(add-hook 'ecky-mode-hook
          (lambda ()
            (setq-local imenu-create-index-function 'ecky-imenu-create-index)))

(provide 'ecky-mode)

;;; ecky-mode.el ends here
