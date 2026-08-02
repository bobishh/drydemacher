#!/usr/bin/env node
/**
 * BDD test for editor support (Emacs and VS Code)
 * Tests JSON parse, VS Code registration, TextMate patterns/scopes, and Emacs font-lock
 */

import { readFileSync, existsSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = join(__dirname, '..');
const EDITORS_DIR = join(ROOT_DIR, 'editors');
const FIXTURE_PATH = join(EDITORS_DIR, 'fixtures', 'full-syntax.ecky');
const EMACS_MODE_PATH = join(EDITORS_DIR, 'emacs', 'ecky-mode.el');
const VSCODE_PACKAGE_PATH = join(EDITORS_DIR, 'vscode', 'package.json');
const VSCODE_SYNTAX_PATH = join(EDITORS_DIR, 'vscode', 'syntaxes', 'ecky.tmLanguage.json');
const RUST_SOURCE_PATH = join(ROOT_DIR, 'src-tauri', 'src', 'ecky_language_surface.rs');

// Parse canonical arrays from Rust source
function parseRustArrays(rustSource) {
  const arrays = {};
  
  // Match pub const NAME: &[&str] = &[...];
  const constRegex = /pub\s+const\s+(\w+):\s+&\[&str\]\s*=\s*&\[([^\]]+)\]/g;
  let match;
  
  while ((match = constRegex.exec(rustSource)) !== null) {
    const name = match[1];
    const valuesStr = match[2];
    
    // Parse the array values
    const values = [];
    const valueRegex = /"([^"]+)"/g;
    let valueMatch;
    
    while ((valueMatch = valueRegex.exec(valuesStr)) !== null) {
      values.push(valueMatch[1]);
    }
    
    arrays[name] = values;
  }
  
  return arrays;
}

// Load and parse Rust source
let CANONICAL_ARRAYS;
try {
  const rustSource = readFile(RUST_SOURCE_PATH);
  CANONICAL_ARRAYS = parseRustArrays(rustSource);
} catch (error) {
  console.error('Failed to parse Rust source:', error.message);
  process.exit(1);
}

// Verify required arrays exist
const REQUIRED_ARRAYS = [
  'MODEL_CLAUSES', 'MODEL_WRAPPERS', 'EXPRESSION_FORMS',
  'NUMERIC_HELPERS', 'POINT_LIST_HELPERS', 'BOOLEAN_HELPERS',
  'CAD_OPS_PORTABLE', 'ECKY_RUST_ONLY_CAD_OPS', 'ECKY_RUST_DIRECT_ONLY_CAD_OPS',
  'WALL_PATTERN_MODES'
];

const missingArrays = REQUIRED_ARRAYS.filter(name => !CANONICAL_ARRAYS[name]);
if (missingArrays.length > 0) {
  console.error(`Missing canonical arrays: ${missingArrays.join(', ')}`);
  process.exit(1);
}

// Required symbols that must be present
const REQUIRED_SYMBOLS = ['model', 'part', 'define-component', 'box', 'repeat', 'wall-pattern', 'sampled-radial-loft'];

// All canonical symbols (combined)
const ALL_CANONICAL_SYMBOLS = [
  ...CANONICAL_ARRAYS.MODEL_CLAUSES,
  ...CANONICAL_ARRAYS.MODEL_WRAPPERS,
  ...CANONICAL_ARRAYS.EXPRESSION_FORMS,
  ...CANONICAL_ARRAYS.NUMERIC_HELPERS,
  ...CANONICAL_ARRAYS.POINT_LIST_HELPERS,
  ...CANONICAL_ARRAYS.BOOLEAN_HELPERS,
  ...CANONICAL_ARRAYS.CAD_OPS_PORTABLE,
  ...CANONICAL_ARRAYS.ECKY_RUST_ONLY_CAD_OPS,
  ...CANONICAL_ARRAYS.ECKY_RUST_DIRECT_ONLY_CAD_OPS,
  ...CANONICAL_ARRAYS.WALL_PATTERN_MODES,
];

class TestResult {
  constructor() {
    this.passed = 0;
    this.failed = 0;
    this.errors = [];
  }

  pass(message) {
    this.passed++;
    console.log(`✓ ${message}`);
  }

  fail(message, error) {
    this.failed++;
    this.errors.push({ message, error });
    console.error(`✗ ${message}`);
    if (error) {
      console.error(`  ${error}`);
    }
  }

  summary() {
    console.log(`\n${this.passed} passed, ${this.failed} failed`);
    if (this.errors.length > 0) {
      console.log('\nFailures:');
      this.errors.forEach(({ message, error }) => {
        console.log(`  - ${message}`);
        if (error) console.log(`    ${error}`);
      });
    }
    return this.failed === 0;
  }
}

function readFile(path) {
  if (!existsSync(path)) {
    throw new Error(`File not found: ${path}`);
  }
  return readFileSync(path, 'utf-8');
}

function testFixtureExists(result) {
  try {
    const content = readFile(FIXTURE_PATH);
    result.pass('Fixture file exists and is readable');
    return content;
  } catch (error) {
    result.fail('Fixture file exists and is readable', error.message);
    return null;
  }
}

function testRequiredSymbolsInFixture(fixtureContent, result) {
  if (!fixtureContent) {
    result.fail('Fixture contains required symbols', 'Fixture content is null');
    return;
  }

  const missingSymbols = REQUIRED_SYMBOLS.filter(sym => !fixtureContent.includes(sym));
  if (missingSymbols.length > 0) {
    result.fail(`Fixture contains required symbols`, `Missing: ${missingSymbols.join(', ')}`);
  } else {
    result.pass(`Fixture contains required symbols: ${REQUIRED_SYMBOLS.join(', ')}`);
  }
}

function testCanonicalSymbolsInFixture(fixtureContent, result) {
  if (!fixtureContent) {
    result.fail('Fixture contains all canonical symbols', 'Fixture content is null');
    return;
  }

  const missingSymbols = ALL_CANONICAL_SYMBOLS.filter(sym => !fixtureContent.includes(sym));
  if (missingSymbols.length > 0) {
    result.fail(`Fixture contains all canonical symbols`, `Missing: ${missingSymbols.join(', ')}`);
  } else {
    result.pass(`Fixture contains all ${ALL_CANONICAL_SYMBOLS.length} canonical symbols`);
  }
}

function testSyntaxElementsInFixture(fixtureContent, result) {
  if (!fixtureContent) {
    result.fail('Fixture contains syntax elements', 'Fixture content is null');
    return;
  }

  const tests = [
    { name: 'comments', pattern: /;.*$/m, shouldMatch: true },
    { name: 'strings', pattern: /"[^"]*"/, shouldMatch: true },
    { name: 'boolean #t', pattern: /#t(?![\w-])/, shouldMatch: true },
    { name: 'boolean #f', pattern: /#f(?![\w-])/, shouldMatch: true },
    { name: 'numbers', pattern: /-?\d+\.?\d*(?:[eE][+-]?\d+)?/, shouldMatch: true },
    { name: 'colon keywords', pattern: /:[a-z-]+/, shouldMatch: true },
    { name: 'model clause', pattern: /\(model\s/, shouldMatch: true },
    { name: 'part definition', pattern: /\(part\s+/, shouldMatch: true },
    { name: 'define', pattern: /\(define\s+/, shouldMatch: true },
    { name: 'lambda', pattern: /\(lambda\s+/, shouldMatch: true },
    { name: 'box function', pattern: /\(box\s+/, shouldMatch: true },
    { name: 'repeat function', pattern: /\(repeat\s+/, shouldMatch: true },
    { name: 'wall-pattern function', pattern: /\(wall-pattern\s+/, shouldMatch: true },
    { name: 'sampled-radial-loft function', pattern: /\(sampled-radial-loft\s+/, shouldMatch: true },
  ];

  tests.forEach(({ name, pattern, shouldMatch }) => {
    const matches = fixtureContent.match(pattern);
    if (shouldMatch && !matches) {
      result.fail(`Fixture contains ${name}`, `Pattern ${pattern} did not match`);
    } else if (!shouldMatch && matches) {
      result.fail(`Fixture should not contain ${name}`, `Pattern ${pattern} matched unexpectedly`);
    } else {
      result.pass(`Fixture contains ${name}`);
    }
  });
}

function testVSCodePackageJson(result) {
  try {
    const content = readFile(VSCODE_PACKAGE_PATH);
    const pkg = JSON.parse(content);

    // Test basic structure
    if (pkg.name !== 'ecky-lang') {
      result.fail('VS Code package.json has correct name', `Expected "ecky-lang", got "${pkg.name}"`);
    } else {
      result.pass('VS Code package.json has correct name');
    }

    if (!pkg.contributes?.languages?.[0]) {
      result.fail('VS Code package.json has language contribution', 'Missing languages contribution');
      return;
    }

    const lang = pkg.contributes.languages[0];
    if (lang.id !== 'ecky') {
      result.fail('VS Code language has correct id', `Expected "ecky", got "${lang.id}"`);
    } else {
      result.pass('VS Code language has correct id');
    }

    if (!lang.extensions?.includes('.ecky')) {
      result.fail('VS Code language has .ecky extension', 'Missing .ecky extension');
    } else {
      result.pass('VS Code language has .ecky extension');
    }

    if (!pkg.contributes?.grammars?.[0]) {
      result.fail('VS Code package.json has grammar contribution', 'Missing grammars contribution');
      return;
    }

    const grammar = pkg.contributes.grammars[0];
    if (grammar.language !== 'ecky') {
      result.fail('VS Code grammar has correct language', `Expected "ecky", got "${grammar.language}"`);
    } else {
      result.pass('VS Code grammar has correct language');
    }

    if (grammar.scopeName !== 'source.ecky') {
      result.fail('VS Code grammar has correct scopeName', `Expected "source.ecky", got "${grammar.scopeName}"`);
    } else {
      result.pass('VS Code grammar has correct scopeName');
    }

    // Check path is relative and valid
    if (!grammar.path || !grammar.path.endsWith('ecky.tmLanguage.json')) {
      result.fail('VS Code grammar has correct path', `Expected path ending with "ecky.tmLanguage.json", got "${grammar.path}"`);
    } else {
      result.pass('VS Code grammar has correct path');
    }

    result.pass('VS Code package.json is valid JSON');
  } catch (error) {
    result.fail('VS Code package.json is valid JSON', error.message);
  }
}

function testVSCodeSyntaxJson(result) {
  try {
    const content = readFile(VSCODE_SYNTAX_PATH);
    const syntax = JSON.parse(content);

    // Test basic structure
    if (syntax.scopeName !== 'source.ecky') {
      result.fail('VS Code syntax has correct scopeName', `Expected "source.ecky", got "${syntax.scopeName}"`);
    } else {
      result.pass('VS Code syntax has correct scopeName');
    }

    if (!Array.isArray(syntax.patterns)) {
      result.fail('VS Code syntax has patterns array', 'Missing or invalid patterns array');
      return;
    }

    result.pass('VS Code syntax has patterns array');

    // Test repository
    if (!syntax.repository) {
      result.fail('VS Code syntax has repository', 'Missing repository');
      return;
    }

    const requiredRepoKeys = [
      'comments', 'strings', 'constants', 'keywords', 'definitions', 'model-clauses',
      'model-wrappers', 'expression-forms', 'numeric-helpers',
      'point-list-helpers', 'boolean-helpers', 'cad-ops', 'wall-pattern-modes',
    ];

    requiredRepoKeys.forEach(key => {
      if (!syntax.repository[key]) {
        result.fail(`VS Code syntax has repository.${key}`, `Missing repository.${key}`);
      } else {
        result.pass(`VS Code syntax has repository.${key}`);
      }
    });

    // Test comment pattern
    if (syntax.repository.comments?.patterns?.[0]?.name !== 'comment.line.semicolon.ecky') {
      result.fail('VS Code syntax has comment pattern with correct name', 'Missing or incorrect comment pattern name');
    } else {
      result.pass('VS Code syntax has comment pattern with correct name');
    }

    // Test string pattern
    if (syntax.repository.strings?.patterns?.[0]?.name !== 'string.quoted.double.ecky') {
      result.fail('VS Code syntax has string pattern with correct name', 'Missing or incorrect string pattern name');
    } else {
      result.pass('VS Code syntax has string pattern with correct name');
    }

    // Test boolean patterns
    const truePattern = syntax.repository.constants?.patterns?.find(p => p.name === 'constant.language.boolean.true.ecky');
    const falsePattern = syntax.repository.constants?.patterns?.find(p => p.name === 'constant.language.boolean.false.ecky');

    if (!truePattern || !truePattern.match || !truePattern.match.includes('#t')) {
      result.fail('VS Code syntax has #t boolean pattern', 'Missing or incorrect #t pattern');
    } else {
      result.pass('VS Code syntax has #t boolean pattern');
    }

    if (!falsePattern || !falsePattern.match || !falsePattern.match.includes('#f')) {
      result.fail('VS Code syntax has #f boolean pattern', 'Missing or incorrect #f pattern');
    } else {
      result.pass('VS Code syntax has #f boolean pattern');
    }

    // Test numeric pattern
    const numericPattern = syntax.repository.constants?.patterns?.find(p => p.name === 'constant.numeric.ecky');
    if (!numericPattern || !numericPattern.match) {
      result.fail('VS Code syntax has numeric pattern', 'Missing numeric pattern');
    } else {
      result.pass('VS Code syntax has numeric pattern');
    }

    // Test keyword (colon) pattern
    const keywordPattern = syntax.repository.keywords?.patterns?.[0];
    if (!keywordPattern || keywordPattern.name !== 'keyword.other.ecky') {
      result.fail('VS Code syntax has keyword (colon) pattern', 'Missing or incorrect keyword pattern');
    } else {
      result.pass('VS Code syntax has keyword (colon) pattern');
    }

    // Test model clauses pattern
    const modelClausesPattern = syntax.repository['model-clauses']?.patterns?.[0];
    if (!modelClausesPattern || modelClausesPattern.name !== 'keyword.control.model-clause.ecky') {
      result.fail('VS Code syntax has model-clauses pattern', 'Missing or incorrect model-clauses pattern');
    } else {
      result.pass('VS Code syntax has model-clauses pattern');
    }

    // Test CAD ops pattern includes required symbols
    const cadOpsPattern = syntax.repository['cad-ops']?.patterns?.[0];
    if (!cadOpsPattern) {
      result.fail('VS Code syntax has cad-ops pattern', 'Missing cad-ops pattern');
    } else {
      result.pass('VS Code syntax has cad-ops pattern');
      const patternStr = cadOpsPattern.match;
      const requiredInPattern = ['box', 'repeat', 'wall-pattern', 'sampled-radial-loft'];
      const missingInPattern = requiredInPattern.filter(sym => !patternStr.includes(sym));
      if (missingInPattern.length > 0) {
        result.fail(`VS Code cad-ops pattern includes required symbols`, `Missing: ${missingInPattern.join(', ')}`);
      } else {
        result.pass(`VS Code cad-ops pattern includes required symbols: ${requiredInPattern.join(', ')}`);
      }
    }

    // Test wall-pattern-modes pattern
    const wallPatternModesPattern = syntax.repository['wall-pattern-modes']?.patterns?.[0];
    if (!wallPatternModesPattern) {
      result.fail('VS Code syntax has wall-pattern-modes pattern', 'Missing wall-pattern-modes pattern');
    } else {
      result.pass('VS Code syntax has wall-pattern-modes pattern');
    }

    result.pass('VS Code syntax JSON is valid');
  } catch (error) {
    result.fail('VS Code syntax JSON is valid', error.message);
  }
}

function testVSCodePatternCoverage(result) {
  try {
    const syntax = JSON.parse(readFile(VSCODE_SYNTAX_PATH));

    // Helper function to check if a symbol is present in a regex pattern
    // accounting for regex escaping (e.g., 'let*' matches 'let\\*')
    const symbolInPattern = (symbol, patternStr) => {
      // Escape special regex characters in the symbol for matching
      const escapedSymbol = symbol
        .replace(/[-\/\\^$*+?.()|[\]{}]/g, '\\$&');
      
      // Try to find the symbol in the pattern, accounting for possible escaping
      // First try exact match, then try with common escape patterns
      if (patternStr.includes(symbol)) {
        return true;
      }
      
      // For symbols with special chars, check if they're escaped
      if (/[*?+]/.test(symbol)) {
        const escapedInPattern = symbol
          .replace(/\*/g, '\\\*')
          .replace(/\?/g, '\\\?')
          .replace(/\+/g, '\\\+');
        if (patternStr.includes(escapedInPattern)) {
          return true;
        }
      }
      
      // Check if symbol appears as a whole word in a regex alternation
      // by looking for it as part of a larger pattern
      const asRegex = new RegExp(`\\b${escapedSymbol.replace(/[-/\\^$*+?.()|[\]{}]/g, '\\$&')}\\b`);
      return asRegex.test(patternStr);
    };

    // Test coverage of canonical arrays
    const coverageTests = [
      { name: 'MODEL_CLAUSES', symbols: CANONICAL_ARRAYS.MODEL_CLAUSES, repoKey: 'model-clauses' },
      { name: 'MODEL_WRAPPERS', symbols: CANONICAL_ARRAYS.MODEL_WRAPPERS, repoKey: 'model-wrappers' },
      { name: 'EXPRESSION_FORMS', symbols: CANONICAL_ARRAYS.EXPRESSION_FORMS, repoKey: 'expression-forms' },
      { name: 'NUMERIC_HELPERS', symbols: CANONICAL_ARRAYS.NUMERIC_HELPERS, repoKey: 'numeric-helpers' },
      { name: 'POINT_LIST_HELPERS', symbols: CANONICAL_ARRAYS.POINT_LIST_HELPERS, repoKey: 'point-list-helpers' },
      { name: 'BOOLEAN_HELPERS', symbols: CANONICAL_ARRAYS.BOOLEAN_HELPERS, repoKey: 'boolean-helpers' },
      { name: 'WALL_PATTERN_MODES', symbols: CANONICAL_ARRAYS.WALL_PATTERN_MODES, repoKey: 'wall-pattern-modes' },
    ];

    coverageTests.forEach(({ name, symbols, repoKey }) => {
      const patterns = syntax.repository[repoKey]?.patterns;
      const patternStr = patterns?.map((pattern) => pattern.match ?? '').join('\n') ?? '';
      if (!patternStr) {
        result.fail(`VS Code pattern covers ${name}`, `Missing pattern for ${repoKey}`);
        return;
      }

      const missing = symbols.filter(sym => !symbolInPattern(sym, patternStr));
      if (missing.length > 0) {
        result.fail(`VS Code pattern covers ${name}`, `Missing: ${missing.join(', ')}`);
      } else {
        result.pass(`VS Code pattern covers ${name} (${symbols.length} symbols)`);
      }
    });

    // CAD ops are combined - test coverage
    const cadOpsPattern = syntax.repository['cad-ops']?.patterns?.[0];
    if (cadOpsPattern && cadOpsPattern.match) {
      const patternStr = cadOpsPattern.match;
      const allCadOps = [
        ...CANONICAL_ARRAYS.CAD_OPS_PORTABLE,
        ...CANONICAL_ARRAYS.ECKY_RUST_ONLY_CAD_OPS,
        ...CANONICAL_ARRAYS.ECKY_RUST_DIRECT_ONLY_CAD_OPS,
      ];
      const missing = allCadOps.filter(sym => !patternStr.includes(sym));
      if (missing.length > 0) {
        result.fail('VS Code cad-ops pattern covers all CAD operations', `Missing: ${missing.join(', ')}`);
      } else {
        result.pass(`VS Code cad-ops pattern covers all CAD operations (${allCadOps.length} symbols)`);
      }
    }

  } catch (error) {
    result.fail('VS Code pattern coverage test', error.message);
  }
}

function testEmacsModeExists(result) {
  try {
    readFile(EMACS_MODE_PATH);
    result.pass('Emacs mode file exists');
  } catch (error) {
    result.fail('Emacs mode file exists', error.message);
  }
}

function testEmacsModeStructure(result) {
  try {
    const content = readFile(EMACS_MODE_PATH);

    // Test for required faces
    const requiredFaces = [
      'ecky-keyword-face',
      'ecky-function-face',
      'ecky-boolean-face',
      'ecky-number-face',
      'ecky-string-face',
      'ecky-comment-face',
      'ecky-keyword-prefix-face',
    ];

    requiredFaces.forEach(face => {
      if (!content.includes(`(defface ${face}`)) {
        result.fail(`Emacs mode defines ${face}`, `Missing defface for ${face}`);
      } else {
        result.pass(`Emacs mode defines ${face}`);
      }
    });

    // Test for required constants
    const requiredConstants = [
      'ecky-model-clauses',
      'ecky-model-wrappers',
      'ecky-expression-forms',
      'ecky-numeric-helpers',
      'ecky-point-list-helpers',
      'ecky-boolean-helpers',
      'ecky-cad-ops-portable',
      'ecky-cad-ops-ecky-rust-only',
      'ecky-wall-pattern-modes',
    ];

    requiredConstants.forEach(constant => {
      if (!content.includes(`(defconst ${constant}`)) {
        result.fail(`Emacs mode defines ${constant}`, `Missing defconst for ${constant}`);
      } else {
        result.pass(`Emacs mode defines ${constant}`);
      }
    });

    // Test for font-lock-keywords function
    if (!content.includes('(defun ecky-font-lock-keywords')) {
      result.fail('Emacs mode has font-lock-keywords function', 'Missing ecky-font-lock-keywords function');
    } else {
      result.pass('Emacs mode has font-lock-keywords function');
    }

    // Test for mode definition
    if (!content.includes('(define-derived-mode ecky-mode')) {
      result.fail('Emacs mode has mode definition', 'Missing define-derived-mode');
    } else {
      result.pass('Emacs mode has mode definition');
    }

    // Test that it derives from prog-mode or scheme-mode
    if (content.includes('(define-derived-mode ecky-mode prog-mode') ||
        content.includes('(define-derived-mode ecky-mode scheme-mode')) {
      result.pass('Emacs mode derives from prog-mode or scheme-mode');
    } else {
      result.fail('Emacs mode derives from prog-mode or scheme-mode', 'Mode does not derive from prog-mode or scheme-mode');
    }

  } catch (error) {
    result.fail('Emacs mode structure test', error.message);
  }
}

function testEmacsFontLockKeywords(result) {
  try {
    const content = readFile(EMACS_MODE_PATH);

    // Test that font-lock-defaults is set correctly
    if (!content.includes("font-lock-defaults")) {
      result.fail('Emacs mode sets font-lock-defaults', 'Missing font-lock-defaults');
    } else {
      result.pass('Emacs mode sets font-lock-defaults');
    }

    // Test that font-lock-defaults calls ecky-font-lock-keywords
    if (content.includes('(list (ecky-font-lock-keywords))')) {
      result.pass('Emacs font-lock-defaults invokes ecky-font-lock-keywords');
    } else {
      result.fail('Emacs font-lock-defaults invokes ecky-font-lock-keywords', 'font-lock-defaults does not invoke ecky-font-lock-keywords');
    }

    // `#` is punctuation in Emacs syntax, so a leading word boundary would
    // never match. Require a delimiter plus a word end after t/f.
    if (content.includes('#\\\\(?:t\\\\|f\\\\)\\\\)\\\\_>')) {
      result.pass('Emacs #t/#f regex uses valid token boundaries');
    } else {
      result.fail('Emacs #t/#f regex uses valid token boundaries', 'Missing delimiter-aware #t/#f pattern');
    }

    // Test for number pattern
    if (!content.includes('font-lock-number-face') && !content.includes('ecky-number-face')) {
      result.fail('Emacs font-lock has number pattern', 'Missing number pattern');
    } else {
      result.pass('Emacs font-lock has number pattern');
    }

    // Test for model clauses pattern
    if (!content.includes('ecky-keywords') && !content.includes('ecky-model-clauses')) {
      result.fail('Emacs font-lock has model clauses pattern', 'Missing model clauses pattern');
    } else {
      result.pass('Emacs font-lock has model clauses pattern');
    }

    // Test for built-in functions pattern
    if (!content.includes('ecky-builtin-functions')) {
      result.fail('Emacs font-lock has built-in functions pattern', 'Missing built-in functions pattern');
    } else {
      result.pass('Emacs font-lock has built-in functions pattern');
    }

    // Test for definition name highlighting
    if (!content.includes('define-component\\\\|define\\\\|part')) {
      result.fail('Emacs font-lock has definition name highlighting', 'Missing define pattern');
    } else {
      result.pass('Emacs font-lock has definition name highlighting');
    }

  } catch (error) {
    result.fail('Emacs font-lock keywords test', error.message);
  }
}

function testEmacsCanonicalCoverage(result) {
  try {
    const content = readFile(EMACS_MODE_PATH);

    // Test that all canonical symbols are in Emacs constants
    const coverageTests = [
      { constName: 'ecky-model-clauses', symbols: CANONICAL_ARRAYS.MODEL_CLAUSES },
      { constName: 'ecky-model-wrappers', symbols: CANONICAL_ARRAYS.MODEL_WRAPPERS },
      { constName: 'ecky-expression-forms', symbols: CANONICAL_ARRAYS.EXPRESSION_FORMS },
      { constName: 'ecky-numeric-helpers', symbols: CANONICAL_ARRAYS.NUMERIC_HELPERS },
      { constName: 'ecky-point-list-helpers', symbols: CANONICAL_ARRAYS.POINT_LIST_HELPERS },
      { constName: 'ecky-boolean-helpers', symbols: CANONICAL_ARRAYS.BOOLEAN_HELPERS },
      { constName: 'ecky-cad-ops-portable', symbols: CANONICAL_ARRAYS.CAD_OPS_PORTABLE },
      { constName: 'ecky-cad-ops-ecky-rust-only', symbols: CANONICAL_ARRAYS.ECKY_RUST_ONLY_CAD_OPS },
      { constName: 'ecky-wall-pattern-modes', symbols: CANONICAL_ARRAYS.WALL_PATTERN_MODES },
    ];

    coverageTests.forEach(({ constName, symbols }) => {
      const constMatch = content.match(new RegExp(`\\(defconst ${constName}\\s+'\\(([^)]+)\\)`));
      if (!constMatch) {
        result.fail(`Emacs ${constName} exists and is non-empty`, `Constant not found or malformed`);
        return;
      }

      const constContent = constMatch[1];
      const missing = symbols.filter(sym => !constContent.includes(`"${sym}"`));

      if (missing.length > 0) {
        result.fail(`Emacs ${constName} contains all symbols`, `Missing: ${missing.join(', ')}`);
      } else {
        result.pass(`Emacs ${constName} contains all symbols (${symbols.length})`);
      }
    });

    // Test that required symbols are present
    const requiredInEmacs = ['model', 'part', 'box', 'repeat', 'wall-pattern', 'sampled-radial-loft'];
    const missingRequired = requiredInEmacs.filter(sym => !content.includes(`"${sym}"`));

    if (missingRequired.length > 0) {
      result.fail('Emacs mode contains required symbols', `Missing: ${missingRequired.join(', ')}`);
    } else {
      result.pass('Emacs mode contains required symbols');
    }

  } catch (error) {
    result.fail('Emacs canonical coverage test', error.message);
  }
}

function testEmacsNoPlaceholderURL(result) {
  try {
    const content = readFile(EMACS_MODE_PATH);

    // Check for placeholder URLs
    const placeholderPatterns = [
      'https://github.com/your-org/',
      'https://github.com/YOUR-ORG/',
      'https://example.com/',
      'YOUR-ORG',
    ];

    const foundPlaceholders = placeholderPatterns.filter(pattern => content.includes(pattern));
    if (foundPlaceholders.length > 0) {
      result.fail('Emacs mode has no placeholder URLs', `Found placeholders: ${foundPlaceholders.join(', ')}`);
    } else {
      result.pass('Emacs mode has no placeholder URLs');
    }

  } catch (error) {
    result.fail('Emacs placeholder URL test', error.message);
  }
}

function testEmacsSyntaxTable(result) {
  try {
    const content = readFile(EMACS_MODE_PATH);

    // Test for syntax table
    if (!content.includes('(defvar ecky-mode-syntax-table')) {
      result.fail('Emacs mode has syntax table', 'Missing syntax table definition');
    } else {
      result.pass('Emacs mode has syntax table');
    }

    // Test for comment syntax (semicolon)
    if (!content.includes('(modify-syntax-entry ?\\; "<"')) {
      result.fail('Emacs syntax table has comment entry', 'Missing semicolon comment syntax');
    } else {
      result.pass('Emacs syntax table has comment entry');
    }

    // Test for string syntax (quote)
    if (!content.includes('(modify-syntax-entry ?\\" "\\"')) {
      result.fail('Emacs syntax table has string entry', 'Missing quote string syntax');
    } else {
      result.pass('Emacs syntax table has string entry');
    }

    // Test for parenthesis pairs
    if (content.includes('(modify-syntax-entry ?\\( "()"') &&
        content.includes('(modify-syntax-entry ?\\) ")("')) {
      result.pass('Emacs syntax table has parenthesis pairs');
    } else {
      result.fail('Emacs syntax table has parenthesis pairs', 'Missing parenthesis syntax');
    }

  } catch (error) {
    result.fail('Emacs syntax table test', error.message);
  }
}

function testEmacsIndentation(result) {
  try {
    const content = readFile(EMACS_MODE_PATH);

    // Test for indentation function
    if (!content.includes('(defun ecky-indent-line')) {
      result.fail('Emacs mode has indentation function', 'Missing ecky-indent-line function');
    } else {
      result.pass('Emacs mode has indentation function');
    }

    // Scheme mode supplies the base indent function; a custom function is
    // also accepted when the mode chooses to override it.
    if (!content.includes('scheme-mode') && !content.includes('indent-line-function')) {
      result.fail('Emacs mode has Scheme indentation', 'Missing scheme-mode or indent-line-function');
    } else {
      result.pass('Emacs mode has Scheme indentation');
    }

    // Test for special handling of model clauses
    if (!content.includes('ecky-model-clauses')) {
      result.fail('Emacs indentation handles model clauses', 'Missing model clauses in indentation logic');
    } else {
      result.pass('Emacs indentation handles model clauses');
    }

  } catch (error) {
    result.fail('Emacs indentation test', error.message);
  }
}

function testEmacsAutoModeAssociation(result) {
  try {
    const content = readFile(EMACS_MODE_PATH);

    // Test for auto-mode-alist
    if (!content.includes('add-to-list') || !content.includes('auto-mode-alist')) {
      result.fail('Emacs mode has auto-mode-alist association', 'Missing auto-mode-alist setup');
    } else {
      result.pass('Emacs mode has auto-mode-alist association');
    }

    // Test that it associates .ecky files
    if (content.includes('.ecky') && content.includes('auto-mode-alist')) {
      result.pass('Emacs mode associates .ecky extension');
    } else {
      result.fail('Emacs mode associates .ecky extension', 'Missing .ecky pattern in auto-mode-alist');
    }

  } catch (error) {
    result.fail('Emacs auto-mode association test', error.message);
  }
}

function testEmacsProvide(result) {
  try {
    const content = readFile(EMACS_MODE_PATH);

    // Test for provide statement
    if (!content.includes('(provide \'ecky-mode)')) {
      result.fail('Emacs mode has provide statement', 'Missing (provide \'ecky-mode)');
    } else {
      result.pass('Emacs mode has provide statement');
    }

  } catch (error) {
    result.fail('Emacs provide test', error.message);
  }
}

function main() {
  console.log('='.repeat(70));
  console.log('Editor Support BDD Test');
  console.log('='.repeat(70));
  console.log();

  const result = new TestResult();

  console.log('Fixture Tests');
  console.log('-'.repeat(70));
  const fixtureContent = testFixtureExists(result);
  testRequiredSymbolsInFixture(fixtureContent, result);
  testCanonicalSymbolsInFixture(fixtureContent, result);
  testSyntaxElementsInFixture(fixtureContent, result);
  console.log();

  console.log('VS Code Tests');
  console.log('-'.repeat(70));
  testVSCodePackageJson(result);
  testVSCodeSyntaxJson(result);
  testVSCodePatternCoverage(result);
  console.log();

  console.log('Emacs Tests');
  console.log('-'.repeat(70));
  testEmacsModeExists(result);
  testEmacsModeStructure(result);
  testEmacsFontLockKeywords(result);
  testEmacsCanonicalCoverage(result);
  testEmacsNoPlaceholderURL(result);
  testEmacsSyntaxTable(result);
  testEmacsIndentation(result);
  testEmacsAutoModeAssociation(result);
  testEmacsProvide(result);
  console.log();

  console.log('='.repeat(70));
  const success = result.summary();
  console.log('='.repeat(70));

  process.exit(success ? 0 : 1);
}

main();
