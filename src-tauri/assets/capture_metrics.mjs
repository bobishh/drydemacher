export const CAPTURE_THRESHOLDS = Object.freeze({
  darkLuminance: 45,
  brightLuminance: 225,
  maximumMotion: 16,
  minimumNovelty: 4,
  holdStillMs: 1_800,
});

function averageDifference(left, right) {
  if (!left || !right || left.length !== right.length) return 100;
  let total = 0;
  for (let index = 0; index < left.length; index += 1) {
    total += Math.abs(left[index] - right[index]);
  }
  return total / left.length;
}

function localLaplacianVariance(signature, width, height) {
  if (width < 5 || height < 5) return 0;
  const fieldWidth = width - 2;
  const fieldHeight = height - 2;
  const integralWidth = fieldWidth + 1;
  const sums = new Float64Array(integralWidth * (fieldHeight + 1));
  const squaredSums = new Float64Array(integralWidth * (fieldHeight + 1));

  for (let y = 0; y < fieldHeight; y += 1) {
    let rowSum = 0;
    let rowSquaredSum = 0;
    for (let x = 0; x < fieldWidth; x += 1) {
      const source = (y + 1) * width + x + 1;
      const laplacian = signature[source - 1] + signature[source + 1]
        + signature[source - width] + signature[source + width]
        - 4 * signature[source];
      rowSum += laplacian;
      rowSquaredSum += laplacian * laplacian;
      const integral = (y + 1) * integralWidth + x + 1;
      sums[integral] = sums[integral - integralWidth] + rowSum;
      squaredSums[integral] = squaredSums[integral - integralWidth] + rowSquaredSum;
    }
  }

  const windowSize = Math.max(4, Math.min(32, Math.floor(Math.min(fieldWidth, fieldHeight) / 3)));
  const step = Math.max(1, Math.floor(windowSize / 2));
  const starts = (length) => {
    const result = [];
    for (let value = 0; value <= length - windowSize; value += step) result.push(value);
    if (result.at(-1) !== length - windowSize) result.push(length - windowSize);
    return result;
  };
  const rectangle = (integral, x, y) => {
    const right = x + windowSize;
    const bottom = y + windowSize;
    return integral[bottom * integralWidth + right]
      - integral[y * integralWidth + right]
      - integral[bottom * integralWidth + x]
      + integral[y * integralWidth + x];
  };

  let maximumVariance = 0;
  const count = windowSize * windowSize;
  for (const y of starts(fieldHeight)) {
    for (const x of starts(fieldWidth)) {
      const sum = rectangle(sums, x, y);
      const squaredSum = rectangle(squaredSums, x, y);
      maximumVariance = Math.max(maximumVariance, squaredSum / count - (sum / count) ** 2);
    }
  }
  return maximumVariance;
}

export function sharpestCandidateIndex(scores) {
  if (scores.length === 0) return -1;
  let selected = 0;
  for (let index = 1; index < scores.length; index += 1) {
    if (Number.isFinite(scores[index]) && scores[index] > scores[selected]) selected = index;
  }
  return selected;
}

export function assessPixels(rgba, width, height, previous, lastAccepted) {
  if (!(width > 1 && height > 1) || rgba.length < width * height * 4) {
    throw new Error('Capture pixel buffer dimensions are invalid.');
  }
  const signature = new Float32Array(width * height);
  let luminance = 0;
  for (let index = 0; index < signature.length; index += 1) {
    const offset = index * 4;
    const value = rgba[offset] * 0.2126 + rgba[offset + 1] * 0.7152 + rgba[offset + 2] * 0.0722;
    signature[index] = value;
    luminance += value;
  }
  luminance /= signature.length;

  let edgeCount = 0;
  let borderEdges = 0;
  let borderSideMask = 0;
  let minX = width;
  let minY = height;
  let maxX = -1;
  let maxY = -1;
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const index = y * width + x;
      const horizontal = x + 1 < width ? Math.abs(signature[index] - signature[index + 1]) : 0;
      const vertical = y + 1 < height ? Math.abs(signature[index] - signature[index + width]) : 0;
      const edge = Math.max(horizontal, vertical);
      if (edge < 18) continue;
      edgeCount += 1;
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
      maxX = Math.max(maxX, x + 1);
      maxY = Math.max(maxY, y + 1);
      if (x <= 1 || y <= 1 || x >= width - 2 || y >= height - 2) {
        borderEdges += 1;
        if (x <= 1) borderSideMask |= 1;
        if (y <= 1) borderSideMask |= 2;
        if (x >= width - 2) borderSideMask |= 4;
        if (y >= height - 2) borderSideMask |= 8;
      }
    }
  }
  const subjectCoverage = edgeCount === 0
    ? 0
    : ((maxX - minX + 1) * (maxY - minY + 1)) / (width * height);

  return {
    luminance,
    sharpness: localLaplacianVariance(signature, width, height),
    subjectCoverage,
    borderContact: edgeCount === 0 ? 0 : borderEdges / edgeCount,
    borderSides: [1, 2, 4, 8].filter(side => (borderSideMask & side) !== 0).length,
    motion: averageDifference(signature, previous),
    novelty: averageDifference(signature, lastAccepted),
    metricDepth: false,
    signature,
  };
}

export function selectCaptureGuidance(metrics, elapsedMs) {
  const t = CAPTURE_THRESHOLDS;
  if (metrics.luminance < t.darkLuminance) {
    return { code: 'tooDark', primary: 'ADD LIGHT', detail: `Luminance ${metrics.luminance.toFixed(0)}`, blocking: true };
  }
  if (metrics.luminance > t.brightLuminance) {
    return { code: 'tooBright', primary: 'REDUCE GLARE', detail: `Luminance ${metrics.luminance.toFixed(0)}`, blocking: true };
  }
  if (metrics.motion > t.maximumMotion) {
    return { code: 'moveSlower', primary: 'MOVE SLOWER', detail: `Motion ${metrics.motion.toFixed(1)}`, blocking: true };
  }
  if (metrics.novelty < t.minimumNovelty) {
    return { code: 'newAngle', primary: 'NEW ANGLE', detail: 'Current view duplicates recent evidence', blocking: true };
  }
  if (elapsedMs < t.holdStillMs) {
    return { code: 'holdStill', primary: 'HOLD STILL', detail: 'Waiting for a stable full-resolution still', blocking: true };
  }
  return { code: 'accepted', primary: 'ACCEPTED', detail: 'Quality gates passed', blocking: false };
}
