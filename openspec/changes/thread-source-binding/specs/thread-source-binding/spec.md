## ADDED Requirements

### Requirement: Every Thread Has a Persistent Editable Source Binding

The system SHALL preserve global Ecky SQLite history and SHALL bind every new
thread to a persistent source folder and `model.ecky` under configured source
root. It SHALL create folder, default source, and sidecar binding manifest on
thread creation. It SHALL retain exact bound path after title/root changes.

#### Scenario: New thread gets source immediately

- GIVEN a configured source root
- WHEN user creates blank thread
- THEN thread folder and `model.ecky` exist under root
- AND `history.sqlite` contains exact source binding
- AND file contains default editable source

### Requirement: Ecky and External Editors Synchronize One Bound File

The system SHALL atomically refresh a clean bound source file after a successful
Ecky-originated version commit. After settled external edit, it SHALL validate,
preview, and commit the source through existing version pipeline, then update
binding digest. It SHALL not create version from unchanged file observation.

#### Scenario: External save becomes version

- GIVEN bound thread with saved version
- WHEN external editor saves valid changed bound source
- THEN watcher commits exactly one new version
- AND Ecky leaves external source bytes unchanged

### Requirement: Pending External Source Is Never Clobbered

The system SHALL compare bound source digest before Ecky-originated source
write. On mismatch it SHALL refuse before writing, keep source/history
unchanged, and surface raw conflict reason.

#### Scenario: Ecky commit meets unsynced edit

- GIVEN bound thread whose file differs from stored digest
- WHEN Ecky attempts to commit different source
- THEN it refuses before overwriting file
- AND no version commits

### Requirement: Source Actions Are Discoverable

The system SHALL show `OPEN FILE` and `REVEAL FOLDER` for bound active thread
in Projects, label Params action `OPEN FILE`, and show raw editor/filesystem
failure instead of only logging it.

#### Scenario: Open source from Projects

- GIVEN active bound thread
- WHEN user opens Projects
- THEN OPEN FILE and REVEAL FOLDER are visible with bound path
- AND OPEN FILE opens that exact source file with OS-configured editor

### Requirement: Agents Receive and Use Bound Source Path

The system SHALL expose bound source path, folder, and sync state in agent
target metadata. Normal agent authoring SHALL edit that file and use source
sync, not export a mirror or use public macro mutation tools.

#### Scenario: Agent edits thread source

- GIVEN agent attached to bound thread
- WHEN it reads metadata and edits supplied source path
- THEN source sync validates and commits edit as new version
- AND no export operation is required
