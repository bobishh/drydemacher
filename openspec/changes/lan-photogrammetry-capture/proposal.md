# Proposal: LAN Photogrammetry Capture

## Intent

Let an iPhone act as a capture head for Ecky without moving model ownership to
the phone. Ecky on Mac starts a local capture session. The phone opens a paired
capture client, presents a continuous camera view with actionable quality and
batch-readiness feedback, and transfers useful source frames. Ecky reconstructs a
dimensioned mesh and imports it through the existing mesh asset boundary.

The user experience is video-like. The reconstruction input remains selected
full-resolution still frames plus capture metadata, not a compressed video file
or an unbounded live stream.

## Scope

- Start, inspect, cancel, and finish a capture session from Ecky.
- Expose a LAN capture URL and QR pairing token.
- Serve a mobile Safari capture client from the running Ecky instance.
- Show live camera preview and actionable too-dark, bounded focus-sampling,
  move-slower, hold-still, accepted-frame, batch-readiness, and transfer feedback.
- Select useful full-resolution frames from continuous camera capture.
- Queue accepted frames through temporary network loss and resume transfer.
- Keep source photos, frame metrics, reconstruction state, and resulting mesh in
  an Ecky-owned capture artifact.
- Persist every changed generated model source, persisted preview draft, and
  Apply source as an immutable model version before validation or reconstruction.
  Capture photos and frame manifests remain capture-run assets, not model
  versions. Latest model append is always head, including failed or stale
  attempts; successful results remain a separate filter.
- Run reconstruction behind a Mac-owned provider interface, with Apple Object
  Capture as the first macOS provider.
- Import the reconstructed triangle mesh through the existing `MeshAsset`
  contract and normal viewport/history lifecycle.
- Define protocol extension points for a native iPhone client supplying ARKit
  pose, calibration, and LiDAR depth without changing session ownership.

## Out Of Scope

- A scan-specific model editor or separate scan document type.
- Treating reconstructed mesh faces as source-backed BRep parameters.
- Streaming every camera frame to Mac or using compressed video as canonical
  reconstruction input.
- Cloud relay, account login, or internet dependency.
- Building the native iPhone LiDAR client in the first implementation.
- Claiming thread, hole, or sub-millimeter dimensional accuracy from capture.

## Product Direction

Phone is a replaceable capture client. Mac/Ecky is canonical backend. A Safari
client provides the first usable path on any modern phone. A later native iPhone
client adds precise pose/depth samples through the same versioned session
protocol. Reconstructed output is an ordinary mesh artifact; existing mesh
editing and future unified direct manipulation apply without scan-only UI.

## Proof Gates

- A real phone on the same LAN can pair from a QR URL without cloud service.
- Camera denial shows the raw browser failure and leaves session recoverable.
- Live capture visibly reports light, bounded relative focus selection, motion,
  and transfer state without universal blur or unsupported distance thresholds.
- Continuous movement accepts useful full-resolution frames without uploading
  every preview frame.
- Network loss preserves queued frames and reconnect resumes idempotently.
- Reconstruction failure preserves source frames and exposes raw provider error.
- Failed, pending, or stale capture mutations remain queryable as head with raw
  status/evidence; no version conflict or force decision discards them.
- Successful reconstruction enters Ecky through `MeshAsset`, then follows normal
  preview and explicit Apply/Commit semantics.
