import type { CaptureDeviationDisplaySample } from '../tauri/contracts';

export type CaptureDeviationClassification = 'within' | 'near' | 'outlier';

export type CaptureDeviationDisplayPoint = CaptureDeviationDisplaySample & {
  classification: CaptureDeviationClassification;
  color: string;
};

const DEVIATION_COLORS: Record<CaptureDeviationClassification, string> = {
  within: '#52c878',
  near: '#e2b24c',
  outlier: '#ef5b5b',
};

export function buildCaptureDeviationDisplayPoints(
  samples: CaptureDeviationDisplaySample[],
  outlierThresholdMm: number,
): CaptureDeviationDisplayPoint[] {
  if (!Number.isFinite(outlierThresholdMm) || outlierThresholdMm <= 0) {
    throw new Error('Deviation display threshold must be finite and positive.');
  }
  return samples.map((sample) => {
    if (
      !Number.isSafeInteger(sample.sourceVertexIndex)
      || sample.sourceVertexIndex < 0
      || !Number.isFinite(sample.distanceMm)
      || sample.distanceMm < 0
      || sample.localPositionMm.some(value => !Number.isFinite(value))
    ) {
      throw new Error('Deviation display sample must contain finite non-negative source evidence.');
    }
    const classification: CaptureDeviationClassification = sample.distanceMm > outlierThresholdMm
      ? 'outlier'
      : sample.distanceMm > outlierThresholdMm * 0.5
        ? 'near'
        : 'within';
    return {
      ...sample,
      localPositionMm: [...sample.localPositionMm] as [number, number, number],
      classification,
      color: DEVIATION_COLORS[classification],
    };
  });
}
