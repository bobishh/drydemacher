import voronoiWebStl from '../models/iphone-17e-voronoi-web.stl?url';
import voronoiWebSourceUrl from '../models/iphone-17e-voronoi-web.ecky?url';
import cellGridStl from '../models/iphone-17e-cell-grid.stl?url';
import cellGridSourceUrl from '../models/iphone-17e-cell-grid.ecky?url';
import oldOpenLatticeStl from '../models/iphone-17e-old-open-lattice.stl?url';
import oldOpenLatticeSourceUrl from '../models/iphone-17e-old-open-lattice.ecky?url';
import oldPerforatedStl from '../models/iphone-17e-old-perforated.stl?url';
import oldPerforatedSourceUrl from '../models/iphone-17e-old-perforated.ecky?url';
import oldCellularReliefStl from '../models/iphone-17e-old-cellular-relief.stl?url';
import oldCellularReliefSourceUrl from '../models/iphone-17e-old-cellular-relief.ecky?url';

export type CaseShowcasePart = {
  label: string;
  url: string;
  downloadName: string;
  color: string;
};

export type CaseShowcaseVariant = {
  id: string;
  kind: 'pattern' | 'earlier';
  label: string;
  title: string;
  note: string;
  artifactId: string;
  sourceUrl: string;
  sourceDownloadName: string;
  parts: CaseShowcasePart[];
};

export const caseShowcaseVariants: CaseShowcaseVariant[] = [
  {
    id: 'voronoi-web',
    kind: 'pattern',
    label: 'VORONOI WEB',
    title: 'iPhone 17e — Voronoi web',
    note: 'Native Voronoi struts. Dense, weird, current.',
    artifactId: 'generated-direct-occt-e098ca48de83',
    sourceUrl: voronoiWebSourceUrl,
    sourceDownloadName: 'iphone-17e-voronoi-web.ecky',
    parts: [{
      label: 'CASE STL',
      url: voronoiWebStl,
      downloadName: 'iphone-17e-voronoi-web.stl',
      color: '#4e735b',
    }],
  },
  {
    id: 'cell-grid',
    kind: 'pattern',
    label: 'CELL GRID',
    title: 'iPhone 17e — Cell grid',
    note: 'Open edge lattice. Lighter, simpler, less polite.',
    artifactId: 'generated-direct-occt-db7279be14a8',
    sourceUrl: cellGridSourceUrl,
    sourceDownloadName: 'iphone-17e-cell-grid.ecky',
    parts: [{
      label: 'CASE STL',
      url: cellGridStl,
      downloadName: 'iphone-17e-cell-grid.stl',
      color: '#667553',
    }],
  },
  {
    id: 'old-open-lattice',
    kind: 'earlier',
    label: 'OPEN LATTICE',
    title: 'Earlier attempt — Open lattice',
    note: 'Before the camera cluster grew opinions.',
    artifactId: 'generated-direct-occt-24400ee27704',
    sourceUrl: oldOpenLatticeSourceUrl,
    sourceDownloadName: 'iphone-17e-old-open-lattice.ecky',
    parts: [{
      label: 'CASE STL',
      url: oldOpenLatticeStl,
      downloadName: 'iphone-17e-old-open-lattice.stl',
      color: '#596b4b',
    }],
  },
  {
    id: 'old-perforated',
    kind: 'earlier',
    label: 'PERFORATED',
    title: 'Earlier attempt — Perforated',
    note: 'A deterministic CSG phase. Expensive lesson included.',
    artifactId: 'generated-direct-occt-cef64e4035bf',
    sourceUrl: oldPerforatedSourceUrl,
    sourceDownloadName: 'iphone-17e-old-perforated.ecky',
    parts: [{
      label: 'CASE STL',
      url: oldPerforatedStl,
      downloadName: 'iphone-17e-old-perforated.stl',
      color: '#5e6650',
    }],
  },
  {
    id: 'old-cellular-relief',
    kind: 'earlier',
    label: 'CELLULAR RELIEF',
    title: 'Earlier attempt — Cellular relief',
    note: 'The mesh-heavy ancestor. It survived.',
    artifactId: 'generated-direct-occt-63d6aadcda79',
    sourceUrl: oldCellularReliefSourceUrl,
    sourceDownloadName: 'iphone-17e-old-cellular-relief.ecky',
    parts: [{
      label: 'CASE STL',
      url: oldCellularReliefStl,
      downloadName: 'iphone-17e-old-cellular-relief.stl',
      color: '#4f6250',
    }],
  },
];

export const currentPatternVariants = caseShowcaseVariants.filter((variant) => variant.kind === 'pattern');
export const earlierCaseVariants = caseShowcaseVariants.filter((variant) => variant.kind === 'earlier');
