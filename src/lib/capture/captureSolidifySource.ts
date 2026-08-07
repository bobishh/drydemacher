import type { MacroAstSourceMapEntry } from '../macroAstMap';

function stringIndexAtByteOffset(source: string, byteOffset: number): number {
  const bytes = new TextEncoder().encode(source);
  if (byteOffset < 0 || byteOffset > bytes.length) {
    throw new Error(`Capture model AST range ends outside source (${byteOffset}/${bytes.length}).`);
  }
  return new TextDecoder('utf-8', { fatal: true }).decode(bytes.slice(0, byteOffset)).length;
}

function capturePartId(captureId: string): string {
  const suffix = captureId.toLowerCase().replace(/[^a-z0-9_]+/g, '_').replace(/^_+|_+$/g, '');
  return `capture_${suffix || 'scan'}`;
}

export function buildCaptureSolidifySource(
  source: string,
  sourceNodes: MacroAstSourceMapEntry[],
  stlPath: string,
  captureId: string,
  scale: number,
): string {
  if (!Number.isFinite(scale) || scale <= 0) throw new Error('Capture scale must be greater than zero.');
  const id = capturePartId(captureId);
  const scaleId = `${id.replace(/^capture_/, 'capture_scale_')}`;
  const scaleLiteral = Number(scale.toPrecision(8)).toString();
  const parameter = `(params (number ${scaleId} ${scaleLiteral} :label "Capture scale" :min 0.001 :max 2 :step 0.001))`;
  const solid = `(solidify (import-stl ${JSON.stringify(stlPath)}))`;
  const part = `(part ${id} (scale ${scaleId} ${scaleId} ${scaleId} ${solid}))`;
  if (!source.trim()) return `(model\n  ${parameter}\n  ${part})`;

  const model = sourceNodes.find((node) => node.id === 'model' && node.kind === 'model');
  if (!model) throw new Error('Capture target has no parser-derived model AST range.');
  const modelEnd = stringIndexAtByteOffset(source, model.endByte);
  const insertAt = modelEnd - 1;
  if (insertAt < 0 || source[insertAt] !== ')') {
    throw new Error('Capture model AST range does not end at model closing parenthesis.');
  }
  return `${source.slice(0, insertAt)}\n  ${parameter}\n  ${part}${source.slice(insertAt)}`;
}
