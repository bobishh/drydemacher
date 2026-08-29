# Ecky CAD

Experimental local desktop CAD for technical makers and developers. Describe a part; Ecky produces readable, editable `.ecky` source and renders a CAD solid.

Ecky does not ask a model to emit arbitrary CAD Python. It lowers a small, inspectable modeling language into a fixed CAD operation set, then renders B-rep geometry through Open CASCADE Technology (OCCT). You can change a dimension, review the source, and regenerate the model.

> Current version: **v0.0.1 pre-release**. No packaged application yet. Build from source. Syntax, APIs, and geometry paths may change; verify dimensions and exported geometry before manufacturing.

[Chapters](https://ecky-cad.com/docs/chapters/) · [Function reference](https://ecky-cad.com/docs/) · [VS Code support](editors/vscode/README.md) · [Emacs support](editors/emacs/README.md)

## What works today

- **AI-assisted or manual authoring** — use a configured Gemini, OpenAI-compatible, or local Ollama provider, or write `.ecky` yourself.
- **Readable model source** — inspect and edit `.ecky` rather than treating an opaque mesh as the model.
- **CAD geometry** — OCCT renders B-rep solids. Change dimensions, inspect geometry, export STEP or STL.
- **Checks and local history** — validate declared requirements before saved versions; source and history remain local.
- **Project-based learning** — six practical chapters connect worked examples, editable starters, hints, and complete solutions.

## Real example: two-thread bicycle bottle holder

The current [bottle holder](sites/landing/src/models/bicycle-bottle-holder.ecky) and its independently authored [frame mount rail](sites/landing/src/models/bottle-holder-frame-mount-rail.ecky) come from two separate Ecky threads. Named parameters own bottle diameter, wall thickness, shared dovetail clearance, frame diameter, clip thickness, and mating rail length. Both native OCCT exports run together in the [landing-page viewer](https://ecky-cad.com/), while each source stays separately inspectable and downloadable.

The source and exports demonstrate the workflow; they do not prove fit on a physical device. Measure and test before manufacturing.

## How it works

`.ecky` crosses three explicit layers:

1. **Surface language** — readable parenthesized source such as `(model (part body ...))`.
2. **Core IR** — compiler-lowered operations with known signatures and backend support.
3. **Geometry backend** — native OCCT renders `.ecky` B-rep geometry; FreeCAD remains optional interop.

Build123d is removed from the runtime and authoring surface. Legacy persisted Build123d identifiers are accepted only for migration to native Ecky settings.

A minimal model:

```scheme
(model
  (params
    (number radius 10 :label "Radius" :min 1 :max 40 :step 1))

  (verify
    (tag preview_exists)
    (metric check (manifest has-model-stl))
    (expect check (= true)))

  (part body
    (sphere radius)))
```

`params` exposes a labeled control. `verify` checks the generated artifact. `part` gives geometry a stable model-level identity. The [Ecky IR Field Guide](docs/books/ecky-ir/index.md) covers the full language.

## Getting started

### Prerequisites

- **Node.js** and **Rust** for the Tauri/Svelte application.
- **FreeCAD** only for the optional FreeCAD backend; `freecadcmd` must be on `PATH`.

Native OCCT and speech runtimes build locally through the prepare scripts. Default OCCT rendering does not require system FreeCAD.

```bash
npm install
npm run runtimes:prepare
npm run tauri dev
```

Open settings, configure one LLM adapter, then create or open a thread. `npm run dev` starts only the Vite frontend and Node server.

Canonical settings live in the platform app-config directory as `config.edn`. `config.json` is legacy import input, not a second writable configuration store.

### CLI tutorial flow

The [`ecky` binary](src-tauri/src/bin/ecky.rs) runs the same source flow as the [Ecky IR Field Guide](docs/books/ecky-ir/index.md): validate a model, inspect lowered backend source, then render an artifact. Build it from the repository with `cargo build --features cli --bin ecky` in `src-tauri/`.

```bash
# Validate the minimal model shown above.
src-tauri/target/debug/ecky check model.ecky

# Inspect FreeCAD interop source without producing geometry.
src-tauri/target/debug/ecky lower --backend freecad model.ecky

# Render one requested artifact. CLI flags override values in params.json.
src-tauri/target/debug/ecky render --backend native model.ecky \
  --params params.json --param radius=12 --stl model.stl --json
```

`check` reports compiler diagnostics without rendering. `lower` supports FreeCAD interop. `render` supports `native` and `freecad`; pass `--bundle-dir out/` when copied runtime bundle files are needed. Exit `2` signals usage, `3` check, `4` lowering, `5` render, and `6` artifact-write failure. Raw compiler and backend details stay on stderr.

## Authoring modes

### API mode

Ecky calls the configured Gemini, OpenAI-compatible, or Ollama endpoint. The response becomes `.ecky` source, then follows the same parse/render/version path as manual source.

### Agent mode through MCP

External agents use Ecky-owned tools and state transitions. Normal sequence:

1. Inspect with `workspace_overview` and related read tools.
2. Validate constraints or an AST patch when applicable.
3. Render a draft with `macro_preview_render`.
4. Check it with `verify_generated_model`; use a screenshot for visual claims.
5. Run `verify_generated_model`; it attaches pass/fail evidence to the already-appended version automatically.

Never write `history.sqlite` directly. Smoke-test preview plus automatic version verification with:

```bash
npm run mcp:smoke -- <thread-id> <path-to-model.ecky> [mcp-url]
```

## Boundaries

Ecky is a pre-release tool, not a production engineering system. Provider output can be wrong. A successful render or verification result does not prove a physical fit. Measure, inspect, and test exported parts before manufacturing.

The language is deliberately bounded: it is not arbitrary Scheme, Python, or a general-purpose CAD scripting host. [The field guide](docs/books/ecky-ir/index.md) and [capability table](src-tauri/src/ecky_ir/backend_capabilities.rs) describe the supported surface.

## Documentation and editor support

- `npm run build:book` builds the EPUB and canonical projections from the file-backed Ecky corpus.
- `npm run build:docs-site` builds the static chapters and function-reference routes.
- `npm run build:mission-assets` regenerates mission images through native OCCT.
- `npm run test:editors` checks the VS Code grammar fixtures and Emacs mode.

## Development

[AGENTS.md](AGENTS.md) contains full conventions. Main checks:

```bash
npm run test:unit
npm run test:component
npm run test:e2e
npm run typecheck
cd src-tauri && cargo check && cargo test
```

For a full Rust run without retaining test/link artifacts in the live Tauri target:

```bash
npm run test:rust:clean
```

This uses a unique temporary Cargo target and removes it on success, failure, or
interrupt. To reclaim accumulated live dev artifacts explicitly, run
`npm run clean:rust-artifacts` while the Tauri dev process is stopped; the next dev
build is cold.

Development uses outer BDD plus inner unit red-green-refactor cycles. Frontend Tauri payloads use `camelCase`; Rust uses `snake_case`; Rust boundary structs translate through `#[serde(rename_all = "camelCase")]`.
