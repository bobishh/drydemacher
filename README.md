# Ecky CAD

Desktop CAD pre-release where LLMs author inspectable `.ecky` source instead of arbitrary modeling scripts. Ecky validates that source, renders it through a geometry backend, checks declared requirements, and records saved versions locally.

> Current version: **v0.0.1**. Syntax, APIs, and geometry paths may break. Verify dimensions and exported geometry before manufacturing.

## Current system

Numbers below describe the v0.0.1 source tree. The linked capability table remains authoritative when counts change.

| Area | Implemented scope | Boundary |
| --- | --- | --- |
| Modeling language | [54 declared Core IR operations](src-tauri/src/ecky_ir/backend_capabilities.rs) behind a parenthesized Scheme surface | Finite operation set; not arbitrary Scheme or Python at kernel boundary |
| Native kernel | 50 operations run directly on OCCT | Text and SVG preprocess; STL import uses interop; native XOR is rejected |
| Geometry backends | 3: native OCCT, build123d, FreeCAD | OCCT is primary; build123d and FreeCAD are interop/parity paths |
| Authoring paths | 2: direct API mode and external-agent MCP mode | Model quality depends on provider, prompt, and verification coverage |
| LLM adapters | 3 families: Gemini, OpenAI-compatible HTTP APIs, local Ollama | “Compatible” describes protocol shape, not equal model behavior |
| Agent persistence gate | 4 stages: inspect → validate → preview → commit | Agent-authored commits require a green verified preview |
| State | Saved versions in SQLite; optional project-folder mirrors for `.ecky` files | Database remains canonical; folders are mirrors, not a second database |

## Measured example

The [iPhone 17e case source](model-runtime/examples/iphone-17e-case-tpu.ecky) contains:

- 30 named parameters covering phone dimensions, clearances, wall thicknesses, ports, and buttons.
- 2 verification clauses: preview existence and zero non-manifold STL edges.
- One 337,584-byte binary [STL export](sites/landing/src/models/iphone-17e-tpu-case.stl), containing 6,750 triangles.

The landing page renders that STL in a live viewer. These numbers prove source and artifact scope. They do not prove physical fit; no fit-test result is recorded here.

## How it works

`.ecky` crosses three explicit layers:

1. **Surface language** — readable parenthesized source such as `(model (part body ...))`.
2. **Core IR** — compiler-lowered operations with known signatures and backend support.
3. **Geometry backend** — native OCCT by default; build123d and FreeCAD for interop and parity work.

A minimal model:

```scheme
(model
  (params
    (number radius 10 :label "Radius" :min 1 :max 40 :step 1))

  (verify
    (tag preview_exists)
    (metric check (manifest has-preview-stl))
    (expect check (= true)))

  (part body
    (sphere radius)))
```

`params` exposes a labeled control. `verify` checks the generated artifact. `part` gives geometry a stable model-level identity. The [Ecky IR Field Guide](docs/books/ecky-ir/index.md) covers the full language.

## How it got here

Repository history shows an iterative boundary change, not a straight-line plan:

| Date | Repository evidence | Change in direction |
| --- | --- | --- |
| 2026-03-26 | First repository snapshot | LLM generated FreeCAD Python macros; headless FreeCAD executed them |
| 2026-03-30 | `Ecky IR` commit | Introduced a project-owned modeling language between model output and kernel |
| 2026-04-14 | `Verification & LLM diet` | Added explicit checks while narrowing what the LLM needed to emit |
| 2026-05-22 | `Book, some more native ecky` | Expanded native rendering and began treating the language reference as a product surface |
| June 2026 | MCP, compiler-error, and native-OCCT changes | Added typed agent tools, self-teaching errors, backend parity checks, and verify-before-commit rules |
| 2026-07-15 | Landing and served-docs changes | Published the v0.0.1 project surface and a real exported model |

Resulting direction: keep generated intent as readable source; constrain kernel input to known operations; test artifacts instead of trusting prose; keep persisted changes behind commands. Several WIP checkpoints preceded this shape. Current architecture should be read as tested evolution, not inevitability.

## Getting started

### Prerequisites

- **Node.js** and **Rust** for the Tauri/Svelte application.
- **Python 3.10+** for interop backends and runtime tooling.
- **FreeCAD** only for the optional FreeCAD backend; `freecadcmd` must be on `PATH`.

Native OCCT, build123d, and speech runtimes build locally through the prepare scripts. Default OCCT rendering does not require system FreeCAD.

```bash
npm install
npm run runtimes:prepare
npm run tauri dev
```

Open settings, configure one LLM adapter, then create or open a thread. `npm run dev` starts only the Vite frontend and Node server.

## Authoring modes

### API mode

Ecky calls the configured Gemini, OpenAI-compatible, or Ollama endpoint. The response becomes `.ecky` source, then follows the same parse/render/version path as manual source.

### Agent mode through MCP

External agents use Ecky-owned tools and state transitions. Normal sequence:

1. Inspect with `workspace_overview` and related read tools.
2. Validate constraints or an AST patch when applicable.
3. Render a draft with `macro_preview_render`.
4. Check it with `verify_generated_model`; use a screenshot for visual claims.
5. Persist only a green draft with `commit_preview_version`.

Never write `history.sqlite` directly. Smoke-test preview-to-commit behavior with:

```bash
npm run mcp:smoke -- <thread-id> <path-to-model.ecky> [mcp-url]
```

## Development

[AGENTS.md](AGENTS.md) contains full conventions. Main checks:

```bash
npm run test:unit
npm run test:e2e
npm run typecheck
cd src-tauri && cargo check && cargo test
```

Development uses outer BDD plus inner unit red-green-refactor cycles. Frontend Tauri payloads use `camelCase`; Rust uses `snake_case`; Rust boundary structs translate through `#[serde(rename_all = "camelCase")]`.
