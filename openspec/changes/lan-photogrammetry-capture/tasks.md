# Tasks: LAN Photogrammetry Capture

## 1. Capture Contract

- [x] 1.1 Add failing backend integration test for session creation, scoped
  pairing token, expiry, cancellation, and state transitions.
- [x] 1.2 Define camelCase Tauri/HTTP contracts and snake_case Rust structs with
  `#[serde(rename_all = "camelCase")]`.
- [x] 1.3 Add capture-session asset layout and persistence through owned backend
  services; no direct SQLite writes.
- [x] 1.4 Add idempotent frame manifest and content-digest reconciliation tests.

## 2. LAN Capture Service

- [x] 2.1 Add failing integration test proving an unpaired client cannot access a
  capture session or unrelated Ecky APIs.
- [x] 2.2 Start a bounded local capture listener on an available port.
- [x] 2.3 Add short-lived QR pairing URL and token revocation.
- [x] 2.4 Add Bonjour/mDNS advertisement with manual URL fallback.
- [x] 2.5 Enforce upload byte limits, image MIME/decode checks, digests, and raw
  error bodies.

## 3. Mobile Safari Capture

- [x] 3.1 Add failing Playwright phone-viewport happy path using synthetic camera
  frames: pair, preview, receive guidance, accept frames, finish.
- [x] 3.2 Add failing camera-denied scenario showing raw browser error and retry.
- [x] 3.3 Add failing disconnect scenario proving accepted frames remain queued
  and resume without duplicate server frames.
- [x] 3.4 Implement full-screen camera surface using Tactical Midnight tokens,
  square borders, stable dimensions, and `overflow: hidden` boundaries.
- [x] 3.5 Implement bounded luminance, relative focus-burst selection, motion,
  novelty, and hold-still assessment without absolute blur or unsupported
  distance gates.
- [x] 3.6 Capture full-resolution stills only after gates pass; retain until
  acknowledgement.
- [x] 3.7 Render one non-overlapping primary instruction plus accepted-frame,
  batch-readiness, and transfer indicators.
- [x] 3.8 Keep the mobile viewport usable across rotation and manage a nonfatal
  screen wake lock for active capture.
- [x] 3.9 Separate certificate and capture QR codes by a scan-safe gap, with a
  stacked narrow-layout fallback.
- [x] 3.10 Rename batch completion to `BUILD PREVIEW`, expose Mac preview state
  without claiming mobile mesh rendering, and resume only after desktop request.
- [x] 3.11 Upload external JPEG/PNG batches through existing bounded frame queue
  without camera permission or a second ingestion contract.

## 4. Evidence And Frame Assessment

- [x] 4.1 Add unit tests for deterministic metric thresholds, relative focus
  selection, and instruction priority.
- [x] 4.2 Add backend tests for decode, duplicate, feature-overlap, and batch
  guidance.
- [x] 4.3 Return actionable too-dark, bounded hold-focus, move-slower,
  hold-still, accepted, duplicate, and preview-readiness guidance. Reserve distance
  guidance for clients declaring reliable depth or segmentation capability.
- [x] 4.4 Keep Safari distance feedback framing-relative and expose capability
  metadata for future metric depth.

## 5. Reconstruction

- [x] 5.1 Define `ReconstructionProvider` under unit tests without Apple types in
  shared contracts.
- [x] 5.2 Add provider-unavailable and provider-failure integration scenarios
  preserving source frames and raw errors.
- [x] 5.3 Implement macOS Apple Object Capture provider with progress and cancel.
- [x] 5.4 Validate output mesh, scale metadata, bounds, and structural evidence.
- [x] 5.5 Route successful output through existing `MeshAsset` interface.

## 6. Workbench Lifecycle

- [x] 6.1 Add failing Playwright flow: start capture, show QR/progress, receive
  reconstructed preview, inspect, Apply/Commit.
- [x] 6.2 Add failure/pending flow preserving active model and source frames.
- [x] 6.3 Historical pre-lossless behavior: reconstruction never auto-committed
  or claimed analytic BRep (version retention superseded by task 6.16).
- [x] 6.4 Keep capture state in Ecky workflow surfaces; no separate agent status
  bar and no scan-specific editor mode.
- [x] 6.5 Bind capture session to target thread/version at creation.
- [x] 6.6 Render reconstructed mesh inside Capture window before Apply.
- [x] 6.7 Offer `SWITCH TO PROJECT` when bound target is not active.
- [x] 6.8 Apply capture through source-backed `solidify(import-stl(...))` AST
  insertion and reject source divergence.
- [x] 6.9 Add pre-Apply uniform scale calibration, scaled bounds, and named Ecky
  scale parameter without rewriting raw reconstruction STL.
- [x] 6.10 Add user-controlled iterative batches that retain source frames and
  rebuild preview after `ADD PHOTOS`; stop presenting frame count as coverage.
- [x] 6.11 Render large reconstruction coordinates with bbox-derived camera
  clipping and preserve originating empty-workspace identity.
- [x] 6.12 Add explicit viewport box crop with move/resize controls, exact AABB
  triangle clipping, derived preview/apply artifact, and immutable raw STL.
- [x] 6.13 Persist capture runs in SQLite with task/version ownership while
  keeping source photos, raw STL, and derived STL in managed filesystem storage.
- [x] 6.14 Render capture runs in task history and reopen them with rotated
  pairing credentials, restored frame manifest, crop, and scale.
- [x] 6.15 Resume historical runs through `ADD PHOTOS` without duplicating run
  identity, and adopt newest pre-durability capture through explicit action.
- [ ] 6.16 Route every changed generated model source, persisted model preview
  draft, and Apply source through one append-before-validation version path;
  retain raw status/evidence and make latest append head even when failed or
  stale. Keep capture assets/run metadata outside model version history.
- [ ] 6.17 Remove capture version-conflict/thread-advanced/force gates while
  preserving geometric validation failures such as bad source digest or crop.

## 7. Native Client Extension Boundary

- [x] 7.1 Add protocol fixtures proving optional intrinsics, pose, and depth
  sidecars do not break Safari clients.
- [x] 7.2 Document capability negotiation needed by a future Swift client.
- [x] 7.3 Keep reconstruction/session ownership on Mac when native metadata is
  present.

## 8. Verification

- [x] 8.1 Run focused frontend unit tests after each inner loop.
- [x] 8.2 Run happy plus camera-denied/network-pending Playwright scenarios.
- [x] 8.3 Run backend integration tests and `cd src-tauri && cargo check`.
- [x] 8.4a Validate physical iPhone Safari certificate trust, pairing, camera,
  frame upload, and successful reconstruction on same LAN.
- [ ] 8.4b Validate physical iPhone Safari rotation, screen wake, and reconnect
  after the latest mobile-client changes.
- [x] 8.5 Run `openspec validate lan-photogrammetry-capture --strict`.
- [ ] 8.6 Add BDD proof that failed reconstruction, pending upload, and stale
  Apply each append one version, become head, and leave earlier successful
  renders available through an explicit successful-only filter.
- [ ] 8.7 Add migration proof for existing capture history: derive deterministic
  append order, preserve all records, and resolve head from latest append.
