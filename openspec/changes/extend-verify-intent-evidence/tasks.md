## 1. Parser and Core IR

- [x] 1.1 Add failing parser and roundtrip tests for optional, reordered, duplicate, unknown, and missing verify sections
- [x] 1.2 Extend Core IR, both parser paths, and canonical emitter for intent, severity, and when

## 2. Verification Runtime and Contracts

- [x] 2.1 Add failing integration tests for warning, skipped, invalid-condition, and backward-compatible outcomes
- [x] 2.2 Evaluate bounded boolean conditions against effective render parameters and apply blocking severity semantics
- [x] 2.3 Extend Rust and generated TypeScript authored-check contracts and MCP projections

## 3. Existing UI

- [x] 3.1 Add controller/component tests for green, red, amber, and neutral authored verification chips
- [x] 3.2 Render typed intent, severity, skipped state, and reason without frontend condition evaluation
- [x] 3.3 Prove real-route happy and warning/skipped UI states with Playwright

## 4. Documentation Hygiene

- [x] 4.1 Document extended verify syntax and semantics in the centralized Ecky corpus
- [x] 4.2 Run the existing documentation freshness pipeline

## 5. Proof

- [x] 5.1 Run OpenSpec strict validation, Rust compile/tests, frontend tests, and contract generation checks
- [x] 5.2 Run documentation freshness checks and record browser proof
