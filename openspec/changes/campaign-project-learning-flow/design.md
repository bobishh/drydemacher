# Design: Campaign Project Learning Flow

## Content authority

Canonical prose lives in `docs/books/ecky-ir/missions/*.md`. Each campaign step
owns a level-two Markdown section with a stable explicit id:

```md
## Join the corner bracket {#worked-bracket}

...step prose...
```

`docs/books/ecky-ir/missions/manifest.edn` owns only campaign and step metadata:
ordered ids, kind, source path, acceptance reference, and reveal policy. It does
not duplicate substantive prose. Tauri packaging copies the canonical campaign
tree into app resources. A Rust campaign-definition service parses EDN with the
existing strict data-only parser, reads Markdown/source/assets from the resource
tree, and rejects missing or duplicate section ids and missing source files. It
never evaluates EDN and writes no JSON mirror.

Typed Tauri commands expose campaign summaries and one requested step payload:
prose, editable source, interaction metadata, bounded previous/next metadata,
and canonical preview identity. Frontend never imports campaign Markdown, Ecky
sources, previews, or EDN and never receives the whole corpus.

Static HTML/EPUB builders render the canonical mission Markdown directly. They
do not need campaign ordering/acceptance metadata, therefore they do not parse
EDN and do not call the runtime service. No second manifest parser exists.

Svelte owns shell and interaction only: current-step request, progress header,
prose projection, editor, preview, action row, and result state. No lesson
paragraphs or whole-corpus loaders live in a component. The existing
`fileBackedMissionBook.ts` corpus glob is removed.

This EDN file is repository-owned campaign definition content. It is not
`app_config_dir/config.edn`; progress, drafts, and active Project state never
enter either definition EDN or config EDN.

## Teaching contract

Steps remain strictly ordered. Completed earlier steps may be revisited through
BACK. Future steps are not addressable from a global table of contents.

Each challenge declares its introduced-form prerequisites in the generated
campaign coverage index. A challenge is invalid when its starter or reference
uses a form that neither the current worked example nor an earlier mission step
has explained. Core IR equivalence remains the acceptance mechanism; grep and
string-count gates are not learner acceptance.

Mission 1 progression is fixed:

1. detached foot and flange: placement plus `compound`;
2. learner overlaps them and changes `compound` to `union`;
3. prose explains `params`, `let*`, `build`, `shape`, and `result` using that code;
4. worked body/lid model introduces separate printable `part` forms and
   `difference`;
5. small male/female coupon introduces named clearance and `if` for snap versus
   bolt geometry;
6. enclosure starter asks only for the two previously demonstrated branch
   selections;
7. revealed solution explains the resulting shared interface;
8. optional mounting/lightening details follow structure.

Later missions use the same rhythm. Large final models are evidence and review,
not unexplained starter code.

## Preview authority

Every source-bearing step has a build-time preview record:

- canonical source digest;
- runtime/backend identity;
- model id;
- verified artifact bundle;
- model STL;
- optional camera framing metadata.

Rust build tooling renders these assets one at a time with a bounded worker and
fails the campaign asset gate if any required preview is absent or digest-stale.
The backend resolves the current step's bundled preview and returns its verified
bundle through the typed step command. Frontend never loads the preview corpus
directly. No render runs merely because a component mounted, a step opened, text
changed, BACK or CONTINUE ran, or the app reloaded.

`RENDER` hashes the current editor source. If it equals the canonical digest,
the bundled artifact is selected. Otherwise the normal immutable artifact cache
is queried with source, params, backend, and runtime identity. Kernel execution
happens only on cache miss. Successful results become reusable; failures remain
uncached and their raw error body is displayed.

## Draft identity and content upgrades

Draft identity is `(campaign definition id, step id, canonical source digest)`.
Changing canonical source creates a new draft slot. Old drafts remain historical
data until normal cleanup but are never projected onto changed source.

Progress ids survive prose-only edits. Removing or replacing a step requires a
manifest migration entry from old id to new id or the first incomplete valid
step. The UI never silently reports completed future steps as earlier history.

## Persistent project surface

Campaign runs remain normalized DB records and never become design threads.
Application navigation persistence stores a typed active Project reference:

```text
kind: design | campaign
id: thread id | campaign run id
view: workbench | campaign
```

Writes flow through Tauri commands. No direct SQLite access, config field,
localStorage, or project folder. Boot loads campaign runs, resolves the saved
active reference, then restores the campaign surface and current step. Missing
or deleted runs fall back to Projects with a raw actionable message.

Projects remains a registered app window. Its visibility and rectangle use the
existing window-layout persistence. Campaign navigation must not close Projects
unless the user explicitly opens/resumes a campaign from it; reload restores the
last persisted visibility.

## UI states

- Explain step: one prose column; no editor, preview, or render instruction.
- Source step, canonical preview ready: prose + editor + visible model.
- Edited source, not rendered: last good preview remains visible with a stale
  marker; `RENDER` enabled.
- Render pending: editor stays usable, prior preview remains, button reports
  pending, duplicate requests coalesce.
- Render failure: prior preview remains and raw backend error is shown.
- Challenge pass/fail: Core IR result appears next to the action; pass advances
  or reveals according to manifest policy.

All major containers retain Tactical Midnight styling, square borders, and
`overflow: hidden`; prose/editor subregions own their scroll.

## Proof boundary

Use the cheapest authoritative gates:

- manifest/Markdown assembly validation;
- preview manifest digest validation without rendering the corpus;
- focused Campaign component behavior for canonical, edited, pending, and error
  states;
- one real-route reload flow for active Campaign and Projects visibility;
- `cargo check` for Rust persistence/command changes;
- no visual sweep and no bulk `ecky check` run.
