<script lang="ts">
  import EckyMascot from './EckyMascot.svelte';
  import CaseWorkbench from './showcase/CaseWorkbench.svelte';

  const repoUrl = 'https://github.com/bobishh/ecky';
  const docsUrl = '/docs/';

  const facts = [
    {
      title: '54 Core IR operations',
      body: '50 run directly on native OCCT. Text and SVG preprocess, import-STL uses interop, and XOR is rejected by native normalization.',
    },
    {
      title: '3 geometry backends',
      body: 'Native OCCT is primary. build123d and FreeCAD remain interop backends for export, import, and parity checks.',
    },
    {
      title: '2 authoring paths',
      body: 'API mode calls a configured LLM provider. MCP mode gives an external agent typed modeling tools.',
    },
    {
      title: '3 provider adapters',
      body: 'Gemini, OpenAI-compatible HTTP APIs, and local Ollama. Model behavior still varies by endpoint and model.',
    },
    {
      title: '4 MCP gates',
      body: 'Inspect, validate, preview, commit. A saved agent-authored version must come from a green verified preview.',
    },
    {
      title: 'Local, inspectable state',
      body: 'SQLite stores saved versions. Project mirrors expose .ecky source as ordinary files without making the folder a second database.',
    },
  ];

  const history = [
    {
      date: 'March 2026',
      title: 'FreeCAD Python macros',
      body: 'First repository snapshot: an LLM generated Python, then headless FreeCAD executed it.',
    },
    {
      date: 'March 2026',
      title: 'Ecky IR',
      body: 'Arbitrary generated scripts gave way to .ecky source and a finite modeling vocabulary.',
    },
    {
      date: 'April 2026',
      title: 'Requirements became code',
      body: 'Verify clauses added machine-checkable expectations alongside geometry.',
    },
    {
      date: 'May 2026',
      title: 'Native rendering expanded',
      body: 'OCCT moved from implementation detail toward the primary kernel; the field guide documented the language.',
    },
    {
      date: 'June 2026',
      title: 'Agent loop gained boundaries',
      body: 'Typed MCP tools, structured authoring errors, backend parity work, and a verify-before-commit rule narrowed failure modes.',
    },
    {
      date: 'July 2026',
      title: 'Public v0.0.1 surface',
      body: 'Static landing, served field guide, native capability table, and real exported models. Still pre-release.',
    },
  ];
</script>

<nav class="nav">
  <div class="nav-inner">
    <a class="brand" href="/">
      <span class="brand-mark">E</span>
      <span class="brand-name">Ecky&nbsp;CAD</span>
    </a>
    <div class="nav-links">
      <a href={docsUrl}>Docs</a>
      <a href={repoUrl} target="_blank" rel="noreferrer">GitHub ↗</a>
    </div>
  </div>
</nav>

<header class="hero" id="case-study">
  <div class="hero-intro">
    <div class="hero-copy">
      <span class="kicker">PROMPT-DRIVEN CAD · V0.0.1 PRE-RELEASE</span>
      <h1 class="hero-title">Write Lisp you don't understand.</h1>
      <p class="hero-lede">Gaslight an LLM until it produces something useful. Burn an irresponsible number of tokens making your dog a funny plastic hat. AI companies love this workflow.</p>
      <p class="hero-summary">
        v0.0.1 pre-release. Describe a part. Argue through the iterations. Keep inspectable <code>.ecky</code> source, native OCCT geometry, and every saved version.
      </p>
      <div class="hero-cta">
        <a class="btn btn-primary" href={repoUrl} target="_blank" rel="noreferrer">Source ↗</a>
        <a class="btn" href={docsUrl}>Docs</a>
      </div>
    </div>
    <div class="hero-mascot" aria-hidden="true">
      <EckyMascot size={190} />
    </div>
  </div>
  <CaseWorkbench />
</header>

<section class="section">
  <div class="section-head">
    <span class="kicker">CURRENT SOURCE TREE</span>
    <h2>Measured scope</h2>
    <p class="section-sub">Counts come from the v0.0.1 capability table and configured adapters. They describe implemented paths, not model quality.</p>
  </div>
  <div class="feature-grid">
    {#each facts as fact}
      <article class="feature-card">
        <h3>{fact.title}</h3>
        <p>{fact.body}</p>
      </article>
    {/each}
  </div>
</section>

<section class="section">
  <div class="section-head">
    <span class="kicker">REPOSITORY HISTORY</span>
    <h2>From Python macros to a constrained CAD language</h2>
    <p class="section-sub">The architecture emerged through working prototypes. Dates and transitions below follow repository history; no clean origin myth.</p>
  </div>
  <div class="feature-grid">
    {#each history as item}
      <article class="feature-card">
        <span class="status">{item.date}</span>
        <h3>{item.title}</h3>
        <p>{item.body}</p>
      </article>
    {/each}
  </div>
</section>

<section class="cta-section">
  <div class="cta-card">
    <h2>Source first. Release later.</h2>
    <p>v0.0.1 can render and version real parts, but no packaged app release exists yet. Inspect the source and verify dimensions before manufacturing.</p>
    <div class="cta-row">
      <a class="btn btn-primary" href={repoUrl} target="_blank" rel="noreferrer">GitHub ↗</a>
      <a class="btn" href={docsUrl}>Docs</a>
    </div>
  </div>
</section>

<footer class="footer">
  <div class="footer-inner">
    <span>Ecky CAD</span>
    <span class="footer-dim">v0.0.1 · 54 operations · 3 backends</span>
    <a href={repoUrl} target="_blank" rel="noreferrer">github.com/bobishh/ecky</a>
  </div>
</footer>
