export function buildCodeWindowTranspilePrompt(source: string): string {
  return [
    'Translate the foreign CAD source below into one parametric Ecky `(model ...)` program.',
    'Infer meaningful parameters and repeated structures. Add authored `(verify ...)` clauses for structural and dialogue requirements.',
    'Treat the attached source block as the source of truth. Return a normal verified Ecky design version, with no prose-only response.',
    '',
    'FOREIGN CAD SOURCE:',
    '```cad',
    source,
    '```',
  ].join('\n');
}
