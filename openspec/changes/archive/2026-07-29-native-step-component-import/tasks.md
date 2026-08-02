## 1. Outer Native STEP Contract

- [x] 1.1 Add one failing integration test: install provenance-backed STEP
  component, live-import alias, place/cut it, render STEP, and assert native
  topology plus absence of FreeCAD invocation.
- [x] 1.2 Add invalid-provenance and mutated-payload cases; confirm failure
  occurs before native execution.

## 2. Package And Resolver Extension

- [x] 2.1 Add failing contract tests for camelCase `geometryProvenance`,
  StepAsset resolution, static zero-argument calls, and STEP lock fields.
- [x] 2.2 Extend package/header contracts and prerequisite resolver payload
  enum; preserve bundle provenance during STEP packaging.

## 3. Direct OCCT ImportStep

- [x] 3.1 Add failing executor tests for valid solid, multiple roots, missing
  file, read/transfer/null/invalid/shell-only failures, and safe path escaping.
- [x] 3.2 Add `OcctOp::ImportStep`, custom-op mapping, runner stage, generated
  `STEPControl_Reader` execution, and SDK required-header/link probe.
- [x] 3.3 Validate admitted shapes before slot publication; never call FreeCAD,
  STL conversion, `solidify`, implicit fuse, or hidden repair.
- [x] 3.4 Feed imported faces/edges through native topology and make executor
  tests green.

## 4. Runtime Truth And Integration

- [x] 4.1 Lower StepAsset aliases to ephemeral `import-step` leaves and make
  the outer placement/boolean test green.
- [x] 4.2 Add conservative representation merge tests across bundle, manifest,
  export, lock, cache identity, and component-origin evidence.

## 5. Gates

- [x] 5.1 Run focused package/resolver/executor/runtime tests, then
  `cd src-tauri && cargo check` and relevant full tests.
- [x] 5.2 Run `openspec validate native-step-component-import --strict` after
  `component-package-imports` validation is green.
