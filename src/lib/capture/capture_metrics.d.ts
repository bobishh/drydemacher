declare module '*capture_metrics.mjs' {
  export type CaptureMetrics = {
    luminance: number;
    sharpness: number;
    subjectCoverage: number;
    borderContact: number;
    borderSides: number;
    motion: number;
    novelty: number;
    metricDepth?: boolean;
    signature?: Float32Array;
  };
  export type CaptureGuidance = {
    code: string;
    primary: string;
    detail: string;
    blocking: boolean;
  };
  export function assessPixels(
    rgba: Uint8ClampedArray,
    width: number,
    height: number,
    previous: Float32Array | null,
    lastAccepted: Float32Array | null,
  ): CaptureMetrics;
  export function selectCaptureGuidance(metrics: CaptureMetrics, elapsedMs: number): CaptureGuidance;
  export function sharpestCandidateIndex(scores: number[]): number;
}
