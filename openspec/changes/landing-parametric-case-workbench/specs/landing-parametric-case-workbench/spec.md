## ADDED Requirements

### Requirement: Curated Case Workbench Vignette

The landing page SHALL present the real iPhone 17e case showcase inside one
bounded artifact frame with a dominant geometry viewport, pattern selection,
earlier saved versions, device identity, and source access.

#### Scenario: Current pattern showcase loads

- GIVEN the landing has two complete published iPhone 17e pattern exports
- WHEN the case-study section renders
- THEN one workbench vignette contains the real iPhone 17e STL viewer
- AND the device is identified without a phone-model selector
- AND exactly two pattern choices are backed by distinct real STL/source pairs
- AND the vignette contains no prompt, Queue, Apply, Commit, dock, or draggable
  window controls

#### Scenario: Fake app chrome is absent

- GIVEN the artifact frame is visible
- WHEN its supporting content renders
- THEN no session history, invented dialogue, source/engine/export badge row,
  parameter-count badge, verification-count badge, or triangle-count badge exists

### Requirement: Manifest-Backed Case Variants

The landing SHALL derive every presented pattern/version, viewer part, source,
label, and download action from one typed static manifest. It SHALL NOT
present incomplete or synthetic presets.

#### Scenario: Current pattern changes

- GIVEN the manifest contains two complete current pattern variants
- WHEN the user chooses another pattern
- THEN viewer parts, pattern label, source, and download URLs update
  atomically from the selected manifest record

#### Scenario: Earlier versions are available

- GIVEN three complete earlier variant records exist
- WHEN the artifact frame renders
- THEN a compact earlier-version strip exposes those three real records
- AND choosing one uses its exact STL/source pair

#### Scenario: Optional artifact is absent

- GIVEN the selected preset has no insert STL or bundle URL
- WHEN download actions render
- THEN no insert or bundle download is offered

### Requirement: Real Static Artifact Downloads

The landing SHALL offer only existing static source and STL/bundle artifacts for
the selected preset. It SHALL NOT claim to generate an export in the browser.

#### Scenario: User downloads a published STL

- GIVEN the selected preset declares a case STL URL
- WHEN the download actions render
- THEN the case link targets that exact static asset
- AND the link carries a meaningful download filename

#### Scenario: User downloads canonical source

- GIVEN the selected preset declares a canonical `.ecky` source URL
- WHEN the source download action renders
- THEN it targets the same source whose full text appears in the inspector

### Requirement: Read-Only Full Source Inspector

The landing SHALL open the selected preset's complete canonical Lisp/Ecky source
in a syntax-highlighted, static macro inspector with Close, Copy Code, and
Download `.ecky` actions only. Its token classes SHALL come from the shared
pure Ecky lexer; the landing SHALL NOT load a CodeMirror or Lezer editor
runtime.

#### Scenario: Code opens complete source

- GIVEN the vignette is focused on a published preset
- WHEN the user activates `SEE CODE`
- THEN an accessible native macro-inspector dialog opens in the viewport top layer
- AND it contains the full canonical source with line numbers
- AND the source is static, cannot be edited, and exposes no editable textbox

#### Scenario: Static renderer shares canonical lexical classes

- GIVEN the desktop app and landing both render the same Ecky source
- WHEN source tokenization runs
- THEN both consume the same pure lexer token spans/classes
- AND the landing does not import CodeMirror or Lezer

#### Scenario: Source is copied

- GIVEN the source inspector is open
- WHEN the user activates `COPY CODE`
- THEN the clipboard receives the complete canonical source
- AND visible copied feedback appears

#### Scenario: Inspector closes from keyboard

- GIVEN the source inspector was opened from `SEE CODE`
- WHEN the user presses Escape
- THEN the inspector closes
- AND keyboard focus returns to the originating `SEE CODE` control

#### Scenario: Inspector remains viewport-bound

- GIVEN the source inspector is open at a 390px viewport
- WHEN its full source exceeds the available height
- THEN the dialog remains completely inside the viewport
- AND page scrolling is locked
- AND keyboard focus remains inside the dialog until it closes

### Requirement: Explicit STL Viewer Lifecycle

The landing STL viewer SHALL expose pending, ready, and failure states for the
active preset and SHALL discard stale loads when preset parts change.

#### Scenario: Real STL becomes ready

- GIVEN the active preset references readable STL parts
- WHEN all parts load
- THEN the pending indicator disappears
- AND the combined assembly is fitted in one shared coordinate frame

#### Scenario: STL load fails

- GIVEN an active part URL cannot load or parse
- WHEN the loader reports its raw failure
- THEN the pending indicator disappears
- AND a visible error identifies the failed asset
- AND a retry control is available

#### Scenario: Preset changes during loading

- GIVEN one preset is still loading
- WHEN another complete preset becomes active
- THEN old geometry and pending state are disposed
- AND late callbacks from the old request do not modify the active viewer

### Requirement: Responsive and Accessible Showcase Boundaries

The vignette and source inspector SHALL remain operable and bounded on desktop
and 390px mobile layouts. All major layout containers SHALL prevent content
bleed with explicit overflow boundaries.

#### Scenario: Narrow viewport renders without horizontal bleed

- GIVEN a 390px viewport
- WHEN the case-study and source inspector render
- THEN the page has no horizontal overflow
- AND viewer, source actions, and close control remain reachable
- AND source scrolling remains inside the inspector

### Requirement: Truthful Hero and Release Actions

The landing SHALL lead with the real case workbench and identify the product as
a pre-release. It SHALL NOT offer an app Download or Releases action while no
packaged application release exists. Static case artifacts remain downloadable.

#### Scenario: No packaged release exists

- GIVEN the repository has no packaged application release
- WHEN the hero and closing call-to-action render
- THEN neither surface offers app Download or Releases actions
- AND source and documentation actions remain available
- AND real case STL downloads remain available inside the workbench

#### Scenario: Mobile navigation remains compact

- GIVEN a 390px viewport
- WHEN the landing navigation renders
- THEN brand, Docs, and GitHub remain on one line
- AND the navigation height does not exceed 68px

### Requirement: Accessible Motion and Efficient Rendering

The landing SHALL honor reduced-motion preference and SHALL NOT run an idle
WebGL render loop for static case geometry.

#### Scenario: Reduced motion is requested

- GIVEN the browser prefers reduced motion
- WHEN the landing renders
- THEN mascot animation remains on one frame
- AND smooth scrolling is disabled

### Requirement: Production Discovery and Response Hardening

The production landing SHALL publish canonical Open Graph and Twitter metadata,
valid crawler discovery files, and baseline browser-security headers.

#### Scenario: Discovery metadata resolves

- GIVEN a crawler requests the production landing
- WHEN the document and crawler assets load
- THEN canonical, Open Graph, Twitter, and theme metadata are present
- AND `robots.txt`, `sitemap.xml`, and the web manifest use their correct content types

#### Scenario: Production response is hardened

- GIVEN a browser requests the production landing over HTTPS
- WHEN Nginx responds
- THEN HSTS, CSP, frame, MIME, referrer, and permissions-policy headers are present
