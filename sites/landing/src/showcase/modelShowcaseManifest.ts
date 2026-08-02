import gilletteBaseStl from '../models/gillette-travel-kit.stl?url';
import gilletteLidStl from '../models/gillette-travel-kit-lid.stl?url';
import gilletteBladeCoverStl from '../models/gillette-travel-kit-blade-cover.stl?url';
import gilletteSourceUrl from '../models/gillette-travel-kit.ecky?url';
import filmBaseStl from '../models/film-scanner-base.stl?url';
import filmLowerGuideStl from '../models/film-scanner-lower-guide.stl?url';
import filmUpperClampStl from '../models/film-scanner-upper-clamp.stl?url';
import filmTunnelStl from '../models/film-scanner-tunnel.stl?url';
import filmHelicoidCoverStl from '../models/film-scanner-helicoid-cover.stl?url';
import filmLensCarrierStl from '../models/film-scanner-lens-carrier.stl?url';
import filmSourceUrl from '../models/film-scanner.ecky?url';
import bottleHolderStl from '../models/bicycle-bottle-holder.stl?url';
import bottleHolderSourceUrl from '../models/bicycle-bottle-holder.ecky?url';
import frameMountRailStl from '../models/bottle-holder-frame-mount-rail.stl?url';
import frameMountRailSourceUrl from '../models/bottle-holder-frame-mount-rail.ecky?url';
import iphoneCaseStl from '../models/iphone-17e-voronoi-case.stl?url';
import iphoneInnerIslandStl from '../models/iphone-17e-camera-inner-island-petg.stl?url';
import iphoneOuterIslandStl from '../models/iphone-17e-camera-outer-snap-island-petg.stl?url';
import iphoneCaseSourceUrl from '../models/iphone-17e-voronoi-case.ecky?url';

export type ModelShowcasePart = {
  label: string;
  url: string;
  downloadName: string;
  color: string;
};

export type ModelShowcaseVariant = {
  id: string;
  label: string;
  title: string;
  note: string;
  sourceUrl: string;
  sourceDownloadName: string;
  sourceLabel?: string;
  companionSources?: Array<{
    label: string;
    url: string;
    downloadName: string;
  }>;
  view: { yaw: number; pitch: number };
  parts: ModelShowcasePart[];
};

export const modelShowcaseVariants: ModelShowcaseVariant[] = [
  {
    id: 'bicycle-bottle-holder',
    label: 'BOTTLE HOLDER',
    title: 'Bicycle bottle holder + frame mount rail',
    note: 'Two source threads. Current cage + independently authored mating frame rail.',
    sourceUrl: bottleHolderSourceUrl,
    sourceDownloadName: 'bicycle-bottle-holder.ecky',
    sourceLabel: 'BOTTLE HOLDER',
    companionSources: [{
      label: 'FRAME MOUNT RAIL',
      url: frameMountRailSourceUrl,
      downloadName: 'bottle-holder-frame-mount-rail.ecky',
    }],
    view: { yaw: 0.45, pitch: -0.5 },
    parts: [
      {
        label: 'BOTTLE HOLDER STL',
        url: bottleHolderStl,
        downloadName: 'bicycle-bottle-holder.stl',
        color: '#b58b32',
      },
      {
        label: 'FRAME MOUNT RAIL STL',
        url: frameMountRailStl,
        downloadName: 'bottle-holder-frame-mount-rail.stl',
        color: '#56745d',
      },
    ],
  },
  {
    id: 'gillette-travel-kit',
    label: 'GILLETTE KIT',
    title: 'Gillette travel kit',
    note: 'Complete 3-print set. Snap shell, sliding lid, blade cover.',
    sourceUrl: gilletteSourceUrl,
    sourceDownloadName: 'gillette-travel-kit.ecky',
    view: { yaw: 0.55, pitch: -0.56 },
    parts: [
      {
        label: 'GILLETTE KIT STL',
        url: gilletteBaseStl,
        downloadName: 'gillette-travel-kit.stl',
        color: '#b58b32',
      },
      {
        label: 'GILLETTE LID STL',
        url: gilletteLidStl,
        downloadName: 'gillette-travel-kit-lid.stl',
        color: '#56745d',
      },
      {
        label: 'BLADE COVER STL',
        url: gilletteBladeCoverStl,
        downloadName: 'gillette-travel-kit-blade-cover.stl',
        color: '#8f6b43',
      },
    ],
  },
  {
    id: 'film-scanner',
    label: 'FILM SCANNER',
    title: 'Film scanner with helicoid',
    note: 'Complete 6-print set. Film path, rail stack, helicoid, lens carrier.',
    sourceUrl: filmSourceUrl,
    sourceDownloadName: 'film-scanner.ecky',
    view: { yaw: 0.62, pitch: -0.54 },
    parts: [
      {
        label: 'SCANNER BASE STL',
        url: filmBaseStl,
        downloadName: 'film-scanner-base.stl',
        color: '#b58b32',
      },
      {
        label: 'LOWER GUIDE STL',
        url: filmLowerGuideStl,
        downloadName: 'film-scanner-lower-guide.stl',
        color: '#657a6a',
      },
      {
        label: 'UPPER CLAMP STL',
        url: filmUpperClampStl,
        downloadName: 'film-scanner-upper-clamp.stl',
        color: '#8c7046',
      },
      {
        label: 'TUNNEL STL',
        url: filmTunnelStl,
        downloadName: 'film-scanner-tunnel.stl',
        color: '#536e61',
      },
      {
        label: 'HELICOID COVER STL',
        url: filmHelicoidCoverStl,
        downloadName: 'film-scanner-helicoid-cover.stl',
        color: '#a47932',
      },
      {
        label: 'LENS CARRIER STL',
        url: filmLensCarrierStl,
        downloadName: 'film-scanner-lens-carrier.stl',
        color: '#745c3b',
      },
    ],
  },
  {
    id: 'iphone-case',
    label: 'PHONE CASE',
    title: 'iPhone 17e — warped-Voronoi TPU + PETG camera island',
    note: 'Complete 3-print set. TPU case + inner PETG seat + outer PETG snap island.',
    sourceUrl: iphoneCaseSourceUrl,
    sourceDownloadName: 'iphone-17e-voronoi-case.ecky',
    view: { yaw: 0.12, pitch: -0.08 },
    parts: [
      {
        label: 'TPU CASE STL',
        url: iphoneCaseStl,
        downloadName: 'iphone-17e-voronoi-case.stl',
        color: '#4e735b',
      },
      {
        label: 'INNER PETG ISLAND STL',
        url: iphoneInnerIslandStl,
        downloadName: 'iphone-17e-camera-inner-island-petg.stl',
        color: '#b58b32',
      },
      {
        label: 'OUTER PETG SNAP ISLAND STL',
        url: iphoneOuterIslandStl,
        downloadName: 'iphone-17e-camera-outer-snap-island-petg.stl',
        color: '#8f6b43',
      },
    ],
  },
];
