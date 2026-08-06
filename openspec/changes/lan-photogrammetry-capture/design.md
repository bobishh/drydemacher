# Design: LAN Photogrammetry Capture

## Goal

Capture one physical object by walking a phone around it, receive useful live
guidance, reconstruct on Mac, and continue working with the resulting mesh in
Ecky.

## Ownership

- Ecky backend owns session identity, pairing, source frame storage,
  reconstruction jobs, canonical mesh artifact, validation, and history.
- Mobile client owns camera permission, live preview, cheap frame metrics,
  full-resolution frame extraction, and retry queue.
- Ecky frontend owns session launch, QR presentation, capture progress,
  reconstruction state, mesh preview, and explicit Apply/Commit actions.
- Reconstruction provider owns photos-to-mesh execution only. Provider-specific
  types do not cross into viewport or mesh-authoring contracts.

## Session Model

```text
CaptureSession
  runId
  targetThreadId + targetMessageId?
  protocolVersion
  state: pairing | capturing | reconstructing | preview | failed | cancelled
  activePairingTokenHash? + expiresAt?
  frameManifest[]
  evidenceSummary
  reconstructionJob
  meshAssetId?
```

```text
CaptureFrame
  frameId
  contentDigest
  capturedAt
  imagePath
  width + height
  clientMetrics: luminance + sharpness + subjectCoverage + motion
  optional pose + intrinsics + depth references
  serverAssessment
```

Raw image bytes live under an Ecky-managed capture-session directory. SQLite is
never written directly. Persisted application references use existing backend
persistence paths and commands.

`capture_runs` is durable SQLite state. It stores task/version binding, lifecycle
state, source snapshot, frame count, raw/derived STL references, crop bounds,
uniform scale, errors, and timestamps. Photo/STL bytes remain filesystem assets.
Pairing tokens, active HTTP clients, cancellation handles, and provider process
handles remain memory-only runtime state.

Task history lists durable capture runs independently from committed model
versions. Opening a run creates a fresh short-lived pairing token, rehydrates its
frame manifest from `captures/<runId>/manifest.json`, and restores raw or cropped
preview. `ADD PHOTOS` appends to that manifest and reconstructs the same run.
Reopening never creates a second run. Missing referenced files produce an exact
error and do not mutate DB state.

Pre-durability capture folders can be adopted explicitly through `OPEN LAST
CAPTURE`. Adoption inspects the newest raw reconstruction STL, binds it to the
current task (or a new deferred task), inserts one durable run, then uses normal
reopen behavior. Subsequent opens resolve by run id, not directory scanning.

## LAN And Pairing

Ecky binds a dedicated local-network HTTP service on an available port and
advertises it through Bonjour/mDNS when available. The desktop displays a QR URL
containing an opaque, short-lived, single-session token. Manual URL entry remains
available when discovery fails.

The token authorizes only its active capture runtime. It expires, can be revoked,
rotates when a durable run is reopened, and
must not grant filesystem, config, history, MCP, or arbitrary API access. Upload
requests are size-limited, MIME-checked, digest-addressed, and idempotent by
`frameId` plus content digest.

First release may use local HTTP where platform camera policy permits it. If
mobile Safari requires a secure context for the selected network address, Ecky
must provide a working trusted HTTPS/development pairing path rather than fake
camera support. Exact transport choice must be proven on physical iPhone Safari.

## Video-Like Capture

The mobile client uses continuous camera preview. It evaluates preview frames at
a bounded rate and captures a full-resolution still only when quality,
movement, novelty, and overlap gates indicate useful evidence. Preview frames
are disposable. Accepted stills are canonical reconstruction inputs.

Fast client checks:

- luminance and clipped-shadow/highlight ratio;
- local Laplacian-variance sharpness score for relative comparison inside a
  bounded stable candidate burst;
- aggregate edge occupancy as non-blocking server metadata;
- visual motion and hold-still window;
- novelty against recently accepted frames;
- local queue and transfer state.

Sharpness has no scene-independent pass/fail threshold. Textureless objects can
produce low absolute scores while focused. Once illumination, framing, motion,
novelty, and hold-still checks permit capture, client retains a bounded burst of
full-resolution candidates and queues highest relative score. `HOLD FOCUS`
describes finite burst collection, not an unbounded rejection state. Future
native client may additionally use AVFoundation autofocus state without changing
this fallback contract.

Mac checks:

- decodability and source dimensions;
- feature count and match overlap with accepted neighbors;
- duplicate rejection;
- appearance-overlap evidence without claiming physical surface coverage;
- provider readiness.

Frame count and appearance similarity are not physical surface coverage. Initial
Safari workflow therefore uses user-controlled batches: collect at least 20
frames, choose `BUILD PREVIEW`, inspect mesh on Mac, then resume the same session
with desktop `ADD PHOTOS` and mobile `RESUME CAMERA` or `UPLOAD PHOTOS`, and
reconstruct again from the complete retained frame set. The phone reports
reconstruction/preview state but does not claim to render the mesh. It does not block or
claim completion from a count-derived percentage. Pose-derived coverage remains
a later native/SfM capability.

Live capture is not the only frame source. Paired browser may select multiple
JPEG/PNG files produced by another phone, camera, or fixed rig. Client decodes
one file at a time for dimensions and bounded metrics, then sends it through the
same digest, idempotency, overlap, manifest, and retry contract as camera stills.
No ZIP-only or unvalidated bulk-ingest path exists.

Feedback is an overlay on the capture view. One primary instruction wins by
severity so messages do not overlap or contradict each other. Aggregate edge
bounds do not identify the subject: background edges can span the frame, while a
valid partial object can touch multiple sides. Safari therefore emits no
blocking distance instruction without depth or an explicit segmentation mask.
A native ARKit/LiDAR client may add measured distance through capability
negotiation.

## Transfer

Control and progress messages use a bounded event channel. Full-resolution
images use individual resumable/idempotent uploads; they are not base64 JSON and
do not share the app's generic small-body request limit. Client retains accepted
frames until server acknowledgement. Reconnect obtains the server frame
manifest and uploads missing digests only.

The protocol is versioned independently from UI implementation:

```text
create session -> pair -> declare client capabilities
preview assessment <-> guidance
declare frame -> upload bytes -> acknowledge digest
finish capture batch -> reconstruct -> inspect -> optionally add photos
cancel/retry
```

Native extensions add camera intrinsics, ARKit transforms, and depth sidecars to
`CaptureFrame`; Safari fields remain valid and required behavior does not fork.

## Reconstruction

`ReconstructionProvider` accepts a capture manifest and session asset directory,
emits progress, and returns either a validated mesh path plus scale metadata or
a raw provider failure. First macOS provider wraps Apple Object Capture. Provider
availability is detected before starting reconstruction.

Source images are retained after failure so provider settings or implementation
can be retried without recapture. Successful output passes mesh decoding,
topology/structural analysis, unit/scale labeling, and the existing `MeshAsset`
boundary. Reconstruction does not imply analytic BRep or editable source
parameters.

## Ecky Lifecycle

Reconstruction completion creates a preview draft. It does not silently replace
the active model or commit history. Capture window renders mesh immediately.
Session remains bound to thread/version selected at start. If another project is
visible when preview arrives, existing Ecky bubble-choice UX offers switch or
stay; no implicit switch occurs. An empty workspace that created the capture
target remains that target despite its deferred thread id and does not receive a
false switch prompt. Apply inserts a captured part through the
parser-reported model AST boundary using `solidify(import-stl(...))`. Source
divergence is a conflict, never a patch against current unrelated screen. Commit
creates a normal version in bound thread. Cancellation removes active
credentials and transient jobs; source asset retention follows an explicit
session action, not hidden cleanup.

Provider unit metadata does not prove physical scale when reconstruction lacks a
metric reference. Capture preview therefore exposes a uniform scale multiplier
and derived millimetre bounds before Apply. Current calibration starts at `0.05`
but remains session-adjustable. Apply writes that value as a named Ecky number
parameter and wraps the captured `solidify(import-stl(...))` part in uniform
`scale`, so later edits use ordinary parameter controls. Raw reconstruction STL
remains immutable.

Capture preview exposes an explicit box crop in the mesh viewport. User moves
and resizes the box, then requests a derived preview. Backend clips triangles
against all six box faces and writes a derived STL. Apply references only the
last successfully previewed crop; changing the box blocks Apply until another
preview succeeds. Empty crops retain the previous preview and expose exact
error. Reset returns to raw preview. Raw reconstruction STL is never overwritten.

## Error Surface

Camera permission denial, insecure-context failure, upload rejection, decode
failure, insufficient source frames, provider absence, provider stderr, and mesh
validation failure remain distinguishable. UI shows raw actionable body from the
responsible browser/backend/provider boundary. Last accepted frame manifest and
last good model remain unchanged.

## BDD Strategy

Outer browser proof uses a real served capture route and synthetic camera stream
for deterministic automation. It covers successful capture and camera-denied or
network-pending state. Physical iPhone Safari proof remains a release gate for
camera permission, pairing URL, rotation, screen wake, and reconnect behavior.

Backend integration tests drive pairing expiry, idempotent upload, manifest
reconciliation, reconstruction provider success/failure, and `MeshAsset`
handoff. Rust changes require `cd src-tauri && cargo check`.
