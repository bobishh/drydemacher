<script lang="ts">
  import { onMount } from 'svelte';
  import EckyMascot from './EckyMascot.svelte';
  import ModelWorkbench from './showcase/ModelWorkbench.svelte';

  const repoUrl = 'https://github.com/bobishh/ecky';
  const chaptersUrl = '/docs/chapters/';
  const referenceUrl = '/docs/';
  const heroInvariants = [
    'keep every dimension named',
    'keep fit relationships explicit',
    'keep source inspectable',
    'keep exports reproducible',
  ];

  let heroInvariantIndex = $state(0);
  const heroInvariant = $derived(heroInvariants[heroInvariantIndex]);

  onMount(() => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    const timer = window.setInterval(() => {
      heroInvariantIndex = (heroInvariantIndex + 1) % heroInvariants.length;
    }, 2_400);
    return () => window.clearInterval(timer);
  });

  const facts = [
    {
      title: 'A solid you can keep editing',
      body: 'Ecky renders B-rep geometry through Open CASCADE Technology (OCCT), a CAD kernel. Change dimensions, inspect faces and edges, then export STEP or STL.',
    },
    {
      title: 'Readable source, bounded vocabulary',
      body: 'The model produces .ecky: a small, inspectable modeling language—not arbitrary generated Python. You can edit the source and rerender the same part.',
    },
    {
      title: 'Checks travel with the geometry',
      body: 'Declare requirements beside a model. Ecky validates source, previews the result, and records the check before an agent-authored version is saved.',
    },
    {
      title: 'Local app, ordinary files',
      body: 'Use a configured Gemini, OpenAI-compatible, or local Ollama provider. Ecky keeps source and saved versions locally; .ecky files remain files you can inspect.',
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
      <a href="#models">Models</a>
      <a href="#learn">Learn</a>
      <a href={repoUrl} target="_blank" rel="noreferrer">GitHub ↗</a>
    </div>
  </div>
</nav>

<header class="hero" id="case-study">
  <div class="hero-intro">
    <div class="hero-copy">
      <span class="kicker">LOCAL DESKTOP AI-ASSISTED CAD · V0.0.1 PRE-RELEASE</span>
      <h1 class="hero-title">Make parts with AI. Keep the model.</h1>
      <p class="hero-invariant-line" aria-label={`make weird shit / ${heroInvariant}`}>
        <span class="hero-invariant-prefix">make weird shit /</span>
        <span class="hero-invariant-value" data-testid="hero-invariant" aria-hidden="true">{heroInvariant}</span>
      </p>
      <p class="hero-lede">Ecky is local desktop CAD for technical makers and developers. Describe a part; inspect or edit the readable <code>.ecky</code> source behind its CAD solid.</p>
      <p class="hero-summary">
        The gallery uses real source and downloadable STLs, not mockups. Experimental pre-release: build from source and verify fit before manufacturing.
      </p>
      <div class="hero-cta">
        <a class="btn btn-primary" href={chaptersUrl}>Read the chapters</a>
        <a class="btn" href="#models">Inspect working models</a>
      </div>
    </div>
    <div class="hero-mascot">
      <EckyMascot size={190} />
    </div>
  </div>
  <div id="models">
    <div class="models-head">
      <span class="kicker">REAL MODELS · SOURCE + STL</span>
      <p>Open the source, orbit the exported parts, and download the same artifacts.</p>
    </div>
    <ModelWorkbench />
  </div>
</header>

<section class="section">
  <div class="section-head">
    <span class="kicker">WHAT MAKES THIS DIFFERENT</span>
    <h2>Review the model. Then change it.</h2>
    <p class="section-sub">A prompt starts the work. Source, solid, and checks stay available when it is time to inspect what actually happened.</p>
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

<section class="cta-section" id="learn">
  <div class="cta-card">
    <span class="kicker">LEARN ECKY</span>
    <h2>Learn Ecky through six practical chapters.</h2>
    <p>The chapters move from a connected bracket through parameters, patterns, named fits, and a multipart mechanism. The function reference stays separate for exact forms and signatures.</p>
    <div class="cta-row">
      <a class="btn btn-primary" href={chaptersUrl}>Read the chapters</a>
      <a class="btn" href={referenceUrl}>Function reference</a>
      <a class="btn" href="/docs/ecky-ir-field-guide.epub" download>Download EPUB</a>
    </div>
  </div>
</section>

<footer class="footer">
  <div class="footer-inner">
    <span>Ecky CAD</span>
    <span class="footer-dim">v0.0.1 pre-release · local desktop CAD</span>
    <a href={repoUrl} target="_blank" rel="noreferrer">github.com/bobishh/ecky</a>
  </div>
</footer>
