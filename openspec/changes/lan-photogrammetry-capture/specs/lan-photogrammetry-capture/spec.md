# LAN Photogrammetry Capture Specification

## ADDED Requirements

### Requirement: Ecky-Owned Capture Session

The system SHALL keep capture session identity, source frames, reconstruction
state, canonical mesh output, and history ownership in Ecky on Mac. A phone
SHALL act as a replaceable capture client and SHALL NOT become a second model or
history authority.

#### Scenario: Phone joins an Ecky session

- **GIVEN** Ecky created an unexpired capture session
- **WHEN** phone presents its scoped pairing token
- **THEN** phone joins only that capture session
- **AND** phone cannot access unrelated files, configuration, history, or APIs

#### Scenario: Pairing token is invalid

- **GIVEN** token is expired, revoked, or belongs to another session
- **WHEN** phone requests capture access
- **THEN** request is rejected with raw reason
- **AND** capture session and active model remain unchanged

#### Scenario: Capture starts from a project thread

- **GIVEN** a project thread or an empty new-project target is active
- **WHEN** Ecky creates capture session
- **THEN** session records target thread and source-version identity
- **AND** later preview cannot patch whichever thread happens to be visible

### Requirement: Lossless Capture Mutation Versions

The system SHALL append one immutable model version for every distinct generated
model source, persisted model preview draft, or Apply source snapshot before
validation or reconstruction. Capture photos, frame manifests, and transient
crop/scale controls SHALL remain capture-run state unless serialized into such a
persisted model draft. The version SHALL retain exact content identity and later
raw status/evidence.
Head SHALL resolve to the latest serialized append regardless of success,
pending state, stale evidence, or artifact availability. Successful renders SHALL
be exposed through a separate filter/projection and SHALL NOT replace head.

#### Scenario: Failed capture mutation becomes head

- **GIVEN** a successful capture preview is head
- **WHEN** a changed candidate or draft is persisted and validation or
  reconstruction fails
- **THEN** changed snapshot is retained as one immutable version
- **AND** raw failure evidence is attached to that version
- **AND** that version is head while the earlier successful render remains
  available through successful filtering

#### Scenario: Stale Apply appends without version conflict

- **GIVEN** an Apply source snapshot was prepared against an older capture/model
  state
- **WHEN** geometric/source validation detects digest or snapshot drift
- **THEN** attempted source snapshot is appended before validation
- **AND** raw stale evidence is attached to it and it becomes head
- **AND** no `conflict`, `threadAdvanced`, or force refusal is emitted
- **AND** prior successful renders remain queryable

#### Scenario: Concurrent capture writers preserve both changes

- **WHEN** two changed capture snapshots are serialized for the same run
- **THEN** each snapshot is appended exactly once
- **AND** later append is head
- **AND** neither append is discarded as a version conflict

### Requirement: Durable Capture History

The system SHALL persist capture-run metadata in SQLite while retaining source
photos and mesh artifacts in Ecky-managed filesystem storage. Pairing credentials
and live job handles SHALL remain transient.

#### Scenario: Capture survives application restart

- **GIVEN** a capture run reached preview with accepted source frames
- **WHEN** Ecky restarts and user opens the owning task history
- **THEN** history lists that capture independently from model versions
- **AND** `OPEN CAPTURE` restores its mesh, scale, and crop selection
- **AND** a new pairing token is issued without changing run identity

#### Scenario: User adds photos to historical capture

- **GIVEN** historical capture manifest contains accepted frames
- **WHEN** user reopens it and chooses `ADD PHOTOS`
- **THEN** Ecky retains existing frames and accepts more through a fresh pairing token
- **AND** reconstruction updates the same durable capture run
- **AND** no duplicate history entry is created

#### Scenario: Historical capture asset is missing

- **WHEN** user opens a capture whose raw STL no longer exists
- **THEN** exact missing path error is shown
- **AND** current viewport and durable capture metadata remain unchanged

#### Scenario: User adopts capture created before durable history

- **GIVEN** capture storage contains a raw reconstruction without a DB run
- **WHEN** user chooses `OPEN LAST CAPTURE`
- **THEN** Ecky binds newest raw reconstruction to current task and inserts one durable run
- **AND** later opens use durable run identity rather than scanning storage again

### Requirement: Video-Like Guided Capture

The mobile client SHALL present continuous camera preview while selecting useful
full-resolution still frames for reconstruction. The system SHALL NOT require
manual shutter operation for every frame and SHALL NOT treat compressed video or
every preview frame as canonical reconstruction input.

#### Scenario: Movement produces useful still frames

- **GIVEN** camera permission is granted and object remains framed
- **WHEN** user walks around object continuously
- **THEN** client evaluates preview frames at bounded rate
- **AND** full-resolution still is accepted only after quality, motion, novelty,
  and overlap gates pass
- **AND** focus quality is selected relatively from a bounded stable candidate
  burst rather than rejected by a scene-independent absolute sharpness threshold
- **AND** accepted frame remains queued until Ecky acknowledges its digest

#### Scenario: Photos come from another device or rig

- **GIVEN** paired client has a set of JPEG or PNG files captured elsewhere
- **WHEN** user selects multiple files through `UPLOAD PHOTOS`
- **THEN** client decodes and transfers files one at a time
- **AND** each file follows same digest, validation, manifest, overlap, and retry contract as camera frames
- **AND** camera permission and wake lock are not requested

#### Scenario: User requests first reconstruction batch

- **GIVEN** at least 20 acknowledged frames exist in current batch
- **WHEN** user chooses `BUILD PREVIEW`
- **THEN** Mac reconstruction starts and phone reports build state
- **AND** preview decision remains in Mac Capture viewport
- **AND** another batch requires desktop `ADD PHOTOS` before phone exposes resume controls

#### Scenario: Camera access fails

- **WHEN** browser denies camera access or rejects insecure capture context
- **THEN** capture view exits pending state
- **AND** raw browser failure is visible with retry action
- **AND** session remains recoverable

#### Scenario: Phone rotates or remains active during capture

- **GIVEN** camera capture is running
- **WHEN** phone rotates or Safari returns from background
- **THEN** camera and controls remain inside the visible viewport without overlap
- **AND** client requests a screen wake lock when supported
- **AND** wake lock is released after capture finishes

### Requirement: Actionable Live Capture Guidance

The capture view SHALL provide non-overlapping live guidance for illumination,
focus sampling, motion, accepted evidence, batch readiness, and transfer state. It
SHALL distinguish heuristics from metric depth and SHALL NOT present unsupported
physical-distance guidance.

#### Scenario: Current frame is unusable

- **GIVEN** preview is too dark or moving too fast
- **WHEN** frame assessment runs
- **THEN** strongest actionable issue is displayed over camera view
- **AND** frame is not accepted while blocking issue remains

#### Scenario: Low-texture object has no universal sharpness score

- **GIVEN** illumination, framing, motion, and novelty gates pass
- **WHEN** absolute sharpness scores remain low because object has little texture
- **THEN** client captures a bounded stable candidate burst
- **AND** sharpest candidate in that burst remains eligible for acceptance
- **AND** `HOLD FOCUS` does not persist after burst completes

#### Scenario: Safari lacks reliable object distance

- **GIVEN** client lacks metric depth and an explicit subject segmentation mask
- **WHEN** frame edges or aggregate occupancy appear high or low
- **THEN** client does not emit blocking `MOVE FARTHER` or `MOVE CLOSER` guidance
- **AND** occupancy remains non-blocking evidence metadata

#### Scenario: Safari cannot verify physical coverage

- **GIVEN** client supplies neither metric pose nor a reconstructed sparse camera path
- **WHEN** accepted frame count reaches batch readiness
- **THEN** capture view offers reconstruction preview without claiming verified coverage
- **AND** user may inspect the mesh and add another photo batch

#### Scenario: User improves a reconstructed preview

- **GIVEN** first batch contains at least the minimum reconstruction frame count
- **WHEN** user decides preview coverage or detail is insufficient
- **THEN** `ADD PHOTOS` resumes the same capture session and pairing URL
- **AND** all accepted source frames remain retained
- **AND** next reconstruction uses the complete old plus new frame set
- **AND** frame count alone is not displayed as verified physical coverage

### Requirement: Resumable LAN Frame Transfer

The system SHALL transfer selected full-resolution frames over local network
with bounded requests, content digests, idempotent frame identities, and client
retry queue. Internet or cloud relay SHALL NOT be required.

#### Scenario: Network disappears during capture

- **GIVEN** phone has unacknowledged accepted frames
- **WHEN** LAN connection becomes unavailable
- **THEN** client retains frames and visibly reports pending transfer
- **AND** capture may continue within bounded local storage

#### Scenario: Client reconnects

- **GIVEN** phone reconnects to same active capture session
- **WHEN** client reconciles its queue with server frame manifest
- **THEN** only missing frame digests upload
- **AND** duplicate requests do not create duplicate canonical frames

### Requirement: Provider-Isolated Mac Reconstruction

Ecky SHALL run photos-to-mesh reconstruction through a Mac-owned provider
interface. Provider-specific types SHALL NOT enter capture, viewport, history,
or mesh-authoring contracts. Apple Object Capture SHALL be the first macOS
provider implementation.

#### Scenario: Reconstruction succeeds

- **GIVEN** capture session contains provider-ready accepted frames
- **WHEN** reconstruction finishes successfully
- **THEN** output mesh and scale metadata are validated
- **AND** mesh enters existing `MeshAsset` interface
- **AND** result becomes preview draft rather than committed history

#### Scenario: Reconstruction fails

- **WHEN** provider is unavailable or returns failure
- **THEN** raw provider error is visible
- **AND** source frames remain available for retry
- **AND** last good model remains the successful-render projection
- **AND** failed reconstruction version and raw provider evidence remain history

### Requirement: Reconstructed Output Is Ordinary Mesh Geometry

The system SHALL treat reconstructed output as imported/generated triangle mesh
with explicit provenance and accuracy limits. It SHALL NOT introduce a scan-only
editor or claim source-backed BRep parameters for reconstructed faces.

#### Scenario: User accepts reconstructed preview

- **GIVEN** reconstructed mesh passed required validation
- **WHEN** user explicitly applies and commits preview
- **THEN** mesh follows normal Ecky artifact/history lifecycle
- **AND** existing or future generic mesh editing operates on it without
  scan-specific model semantics

#### Scenario: Reconstructed mesh enters Ecky source

- **GIVEN** capture remains bound to target thread and source has not diverged
- **WHEN** user applies reconstructed preview
- **THEN** target `.ecky` model gains a source-backed
  `solidify(import-stl(...))` part through parser-derived AST range
- **AND** empty target becomes a minimal model containing only captured
  solidified part
- **AND** commit creates a normal version in bound thread

#### Scenario: Preview completes while another project is active

- **GIVEN** capture is bound to thread A and thread B is visible
- **WHEN** reconstruction enters preview state
- **THEN** Ecky bubble offers `SWITCH TO PROJECT` and `STAY HERE`
- **AND** thread B remains unchanged until user chooses switch

#### Scenario: Preview completes in its originating empty workspace

- **GIVEN** capture created a deferred thread identity from current empty workspace
- **WHEN** reconstruction enters preview state before first commit
- **THEN** Ecky does not offer a project switch
- **AND** Apply adopts deferred identity while preserving visible Capture window

#### Scenario: User crops reconstructed mesh explicitly

- **GIVEN** raw reconstruction includes object and unwanted surrounding geometry
- **WHEN** user moves or resizes `BOX CROP` and requests `PREVIEW CROP`
- **THEN** backend clips mesh triangles against all six box faces
- **AND** backend writes and previews a derived STL inside selected volume
- **AND** Apply references derived STL
- **AND** changing selection blocks Apply until crop preview succeeds
- **AND** raw reconstruction STL remains unchanged

#### Scenario: Box crop contains no mesh

- **WHEN** selected box excludes all mesh triangles
- **THEN** previous preview remains visible
- **AND** Apply remains blocked for unpreviewed selection
- **AND** exact crop error is shown

#### Scenario: User resets box crop

- **GIVEN** Capture currently previews a derived cropped STL
- **WHEN** user chooses `RESET CROP`
- **THEN** Capture restores raw reconstruction preview
- **AND** subsequent Apply references raw reconstruction STL

#### Scenario: Reconstruction lacks trustworthy physical scale

- **GIVEN** provider mesh dimensions do not match a measured object dimension
- **WHEN** user changes uniform capture scale before Apply
- **THEN** Capture preview shows scale-corrected millimetre bounds
- **AND** Apply adds a named scale parameter around the captured
  `solidify(import-stl(...))` part
- **AND** raw reconstruction STL remains unchanged
- **AND** later parameter edits scale captured geometry uniformly

#### Scenario: User selects reconstructed face

- **WHEN** selected triangle lacks exact source binding
- **THEN** it remains mesh geometry
- **AND** system does not synthesize a BRep parameter or editable source node

### Requirement: Native Depth Client Extends Same Protocol

The capture protocol SHALL support optional camera intrinsics, poses, and depth
sidecars so a native iPhone client can improve measurement and derive coverage without
forking session ownership or reconstruction lifecycle.

#### Scenario: Native client supplies LiDAR metadata

- **WHEN** paired client declares pose and depth capabilities
- **THEN** frames may include versioned intrinsics, transforms, and depth
  references
- **AND** Ecky remains canonical session and reconstruction owner
- **AND** Safari clients remain valid without those optional fields
