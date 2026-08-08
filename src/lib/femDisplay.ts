export type FemFieldKind = 'vonMises' | 'displacement';

export type FemDisplayOptions = {
  field: FemFieldKind;
  deformationScale: number;
  showMesh: boolean;
  showOutline: boolean;
  clipFraction: number;
};

export function normalizeFemField(value: number, minimum: number, maximum: number): number {
  if (!Number.isFinite(value) || !Number.isFinite(minimum) || !Number.isFinite(maximum)) return 0;
  if (maximum <= minimum) return 0;
  return Math.max(0, Math.min(1, (value - minimum) / (maximum - minimum)));
}

export function femColorRamp(normalized: number): [number, number, number] {
  const value = Math.max(0, Math.min(1, Number.isFinite(normalized) ? normalized : 0));
  const stops: Array<[number, [number, number, number]]> = [
    [0, [0.08, 0.25, 0.55]],
    [0.35, [0.08, 0.68, 0.72]],
    [0.65, [0.88, 0.73, 0.18]],
    [1, [0.78, 0.12, 0.08]],
  ];
  const upperIndex = stops.findIndex(([position]) => value <= position);
  if (upperIndex <= 0) return stops[0][1];
  const [lowPosition, lowColor] = stops[upperIndex - 1];
  const [highPosition, highColor] = stops[upperIndex];
  const fraction = (value - lowPosition) / (highPosition - lowPosition);
  return lowColor.map((channel, index) => channel + (highColor[index] - channel) * fraction) as [number, number, number];
}
