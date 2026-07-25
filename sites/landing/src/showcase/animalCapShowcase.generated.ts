// GENERATED FILE. Run: node scripts/sync_animal_cap_catalog.mjs
import quaterniusPugPrestaStlUrl from '../../../../catalogs/animal-caps/assets/published/pug-presta-valve-cap.stl?url';
import quaterniusPugPrestaSourceUrl from '../../../../catalogs/animal-caps/assets/published/pug-presta-valve-cap.ecky?url';
import quaterniusPugPrestaPreviewUrl from '../../../../catalogs/animal-caps/assets/published/pug-presta-valve-cap.png?url';

export type AnimalCapShowcaseEntry = {
  id: string;
  displayName: string;
  species: string;
  boreProfileId: string;
  boreAxis: 'x' | 'y' | 'z';
  boreAxisHeightMm: number;
  uniformScale: number;
  license: string;
  sourceAuthor: string;
  sourcePageUrl: string;
  modelId: string;
  verificationStatus: 'passed';
  verifiedTriangleCount: number;
  stlUrl: string;
  stlDownloadName: string;
  sourceUrl: string;
  sourceDownloadName: string;
  previewUrl: string;
};

export const animalCapShowcaseEntries: AnimalCapShowcaseEntry[] = [
  {
    id: "quaternius-pug-presta",
    displayName: "Pug Presta Valve Cap",
    species: "Pug",
    boreProfileId: "presta-blind-bomb-v1",
    boreAxis: "y",
    boreAxisHeightMm: 0,
    uniformScale: 12,
    license: "CC0-1.0",
    sourceAuthor: "Quaternius",
    sourcePageUrl: "https://opengameart.org/content/lowpoly-animated-farm-animal-pack",
    modelId: "generated-direct-occt-d4a899333661",
    verificationStatus: "passed",
    verifiedTriangleCount: 2572,
    stlUrl: quaterniusPugPrestaStlUrl,
    stlDownloadName: "pug-presta-valve-cap.stl",
    sourceUrl: quaterniusPugPrestaSourceUrl,
    sourceDownloadName: "pug-presta-valve-cap.ecky",
    previewUrl: quaterniusPugPrestaPreviewUrl,
  }
];
