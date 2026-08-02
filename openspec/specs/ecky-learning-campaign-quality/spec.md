# ecky-learning-campaign-quality Specification

## Purpose
TBD - created by archiving change repair-ecky-learning-campaign. Update Purpose after archive.
## Requirements
### Requirement: Campaign renders one active learning phase

The campaign SHALL mount only the selected phase. Initial phase SHALL be BRIEF.
Navigation SHALL change visible content rather than scroll a page containing every
phase.

#### Scenario: Mission opens

- **WHEN** a learner opens Level 01
- **THEN** BRIEF is visible
- **AND** STUDY, DECIDE, PRACTICE, TRANSFER, and DEBRIEF content are not visible.

#### Scenario: Learner selects Study

- **WHEN** STUDY is activated
- **THEN** BRIEF content is removed from the visible phase surface
- **AND** exactly one worked-example source block is visible.

### Requirement: Worked examples do not expose mission answers

Every mission SHALL use materially different worked-example and solution source.
BRIEF SHALL state the problem without giving construction steps. Hints SHALL not paste
the exact replacement required by practice.

#### Scenario: Content projection is validated

- **WHEN** mission source checks run
- **THEN** identical or trivially equivalent worked-example/solution source fails
- **AND** answer-bearing brief or hint content fails with the mission id.

### Requirement: Campaign and offline book are distinct products

The campaign SHALL NOT call an EPUB a downloadable campaign. It SHALL expose a
secondary action named `OFFLINE BOOK · EPUB`. Full chapter answers SHALL not render
simultaneously beneath the interactive workbench.

#### Scenario: Campaign header renders

- **WHEN** the campaign loads
- **THEN** `OFFLINE BOOK · EPUB` is visible
- **AND** `DOWNLOAD CAMPAIGN` is absent
- **AND** activating the action downloads the EPUB artifact.

### Requirement: Corner Bracket shows source-bound real geometry

Level 01 SHALL show a non-broken image generated from committed Corner Bracket Ecky
source through native Ecky geometry and the existing Three/WebGL browser renderer.

#### Scenario: Corner Bracket brief renders

- **WHEN** Level 01 BRIEF is visible
- **THEN** a descriptive Corner Bracket image is visible
- **AND** its natural width and height are greater than zero
- **AND** build checks bind the image to canonical Ecky source.

### Requirement: Level 03 teaches a dovetail fit relation

Level 03 SHALL extract its male dovetail rail and female dovetail channel from the
existing production film-adapter subsystem. One named `fit_clearance` relation SHALL
drive both mating sides. Preview-only assembly placement SHALL remain outside
production export geometry. The campaign SHALL NOT introduce a parallel dovetail
implementation or duplicate general CAD-operation tests.

#### Scenario: Dovetail mission loads

- **WHEN** Level 03 is selected
- **THEN** its artifact, objective, practice, and real render describe Dovetail Fit
- **AND** source provenance points to the existing film-adapter implementation
- **AND** ribbed plate is absent from primary campaign content.

#### Scenario: Fit is edited

- **WHEN** the learner changes `fit_clearance`
- **THEN** male/female mating dimensions remain derived from the named relation
- **AND** no second anonymous clearance offset requires editing.

#### Scenario: Extracted fixture is validated

- **WHEN** the production-derived fixture is validated once through MCP or native Ecky
- **THEN** existing runtime/model tests remain authoritative for CAD correctness
- **AND** campaign tests assert only projection, named fit teaching, and asset loading.

#### Scenario: Parts export contract is preserved

- **WHEN** existing production geometry is exported
- **THEN** male and female parts remain separate printable parts
- **AND** preview assembly transforms are not emitted into export geometry.

