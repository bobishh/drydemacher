export type AuthoringGraphTargetKind = 'part' | 'object' | 'group' | 'edge' | 'face';

export type AuthoringGraphInputPort = {
  role: string;
  valueKind: string;
  cardinality: 'one' | 'many';
  childPath: string;
};

export type AuthoringGraphAstNode = {
  path: string;
  stableNodeKey: string;
  kind: string;
  valueKind: string;
  operation?: string | null;
  partId?: string | null;
  sourceAddressable: boolean;
  editableOps: string[];
  nonEditableReason?: string | null;
  childPaths: string[];
  inputPorts: AuthoringGraphInputPort[];
};

export type AuthoringGraphTarget = {
  targetId: string;
  durableTargetId?: string | null;
  canonicalTargetId?: string | null;
  aliasIds?: string[];
  partId: string;
  viewerNodeId: string;
  label: string;
  kind: AuthoringGraphTargetKind;
  parameterKeys?: string[];
  primitiveIds?: string[];
  featureIds?: string[];
  sourceStableNodeKeys?: string[];
  editable: boolean;
  nonEditableReason?: string | null;
};

export type AuthoringGraph = {
  sourceDigest: string;
  coreDigest: string;
  artifactDigest?: string | null;
  astNodes: AuthoringGraphAstNode[];
  features: unknown[];
  dependencies: unknown[];
  constraints: unknown[];
  targets: AuthoringGraphTarget[];
  handles: unknown[];
};

export type AuthoringGraphRequest = {
  source: string;
  modelId?: string | null;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function requireString(record: Record<string, unknown>, key: string, owner: string): string {
  const value = record[key];
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`${owner} requires non-empty camelCase '${key}'.`);
  }
  return value;
}

function optionalStringArray(record: Record<string, unknown>, key: string, owner: string): string[] {
  const value = record[key];
  if (value === undefined) return [];
  if (!Array.isArray(value) || value.some((item) => typeof item !== 'string')) {
    throw new Error(`${owner} requires camelCase '${key}' as a string array.`);
  }
  return value;
}

function decodeTarget(value: unknown, index: number): AuthoringGraphTarget {
  if (!isRecord(value)) {
    throw new Error(`Authoring graph target ${index} must be an object.`);
  }
  const targetId = requireString(value, 'targetId', `Authoring graph target ${index}`);
  requireString(value, 'partId', `Authoring target '${targetId}'`);
  requireString(value, 'viewerNodeId', `Authoring target '${targetId}'`);
  requireString(value, 'label', `Authoring target '${targetId}'`);
  requireString(value, 'kind', `Authoring target '${targetId}'`);
  if (typeof value.editable !== 'boolean') {
    throw new Error(`Authoring target '${targetId}' requires backend-owned boolean 'editable'.`);
  }

  const featureIds = optionalStringArray(value, 'featureIds', `Authoring target '${targetId}'`);
  const sourceStableNodeKeys = optionalStringArray(
    value,
    'sourceStableNodeKeys',
    `Authoring target '${targetId}'`,
  );
  const reason = value.nonEditableReason;
  if (
    reason !== undefined &&
    reason !== null &&
    (typeof reason !== 'string' || reason.trim().length === 0)
  ) {
    throw new Error(`Authoring target '${targetId}' has invalid raw 'nonEditableReason'.`);
  }
  if (value.editable && (featureIds.length === 0 || sourceStableNodeKeys.length === 0)) {
    throw new Error(`Editable authoring target '${targetId}' lacks exact feature and AST binding.`);
  }
  if (!value.editable && (typeof reason !== 'string' || reason.trim().length === 0)) {
    throw new Error(`Non-editable authoring target '${targetId}' lacks raw backend reason.`);
  }
  if (value.editable && reason != null) {
    throw new Error(`Editable authoring target '${targetId}' cannot include 'nonEditableReason'.`);
  }

  return value as AuthoringGraphTarget;
}

function decodeInputPort(value: unknown, owner: string, index: number): AuthoringGraphInputPort {
  if (!isRecord(value)) throw new Error(`${owner} input port ${index} must be an object.`);
  const role = requireString(value, 'role', `${owner} input port ${index}`);
  requireString(value, 'valueKind', `${owner} input port '${role}'`);
  requireString(value, 'childPath', `${owner} input port '${role}'`);
  if (value.cardinality !== 'one' && value.cardinality !== 'many') {
    throw new Error(`${owner} input port '${role}' requires 'one' or 'many' cardinality.`);
  }
  return value as AuthoringGraphInputPort;
}

function decodeAstNode(value: unknown, index: number): AuthoringGraphAstNode {
  if (!isRecord(value)) throw new Error(`Authoring graph AST node ${index} must be an object.`);
  const path = requireString(value, 'path', `Authoring graph AST node ${index}`);
  const owner = `Authoring graph AST node '${path}'`;
  requireString(value, 'stableNodeKey', owner);
  requireString(value, 'kind', owner);
  requireString(value, 'valueKind', owner);
  if (typeof value.sourceAddressable !== 'boolean') {
    throw new Error(`${owner} requires backend-owned boolean 'sourceAddressable'.`);
  }
  const editableOps = optionalStringArray(value, 'editableOps', owner);
  const childPaths = optionalStringArray(value, 'childPaths', owner);
  const rawPorts = value.inputPorts ?? [];
  if (!Array.isArray(rawPorts)) throw new Error(`${owner} requires camelCase 'inputPorts' array.`);
  const nonEditableReason = value.nonEditableReason;
  if (!value.sourceAddressable && (typeof nonEditableReason !== 'string' || !nonEditableReason.trim())) {
    throw new Error(`${owner} requires raw 'nonEditableReason' when non-addressable.`);
  }
  if (value.sourceAddressable && nonEditableReason != null) {
    throw new Error(`${owner} cannot include 'nonEditableReason' when source-addressable.`);
  }
  return {
    ...(value as Omit<AuthoringGraphAstNode, 'editableOps' | 'childPaths' | 'inputPorts'>),
    editableOps,
    childPaths,
    inputPorts: rawPorts.map((port, portIndex) => decodeInputPort(port, owner, portIndex)),
  };
}

export function decodeAuthoringGraph(value: unknown): AuthoringGraph {
  if (!isRecord(value)) {
    throw new Error('Authoring graph must be an object.');
  }
  requireString(value, 'sourceDigest', 'Authoring graph');
  requireString(value, 'coreDigest', 'Authoring graph');
  for (const key of ['astNodes', 'features', 'dependencies', 'constraints', 'targets', 'handles']) {
    if (!Array.isArray(value[key])) {
      throw new Error(`Authoring graph requires camelCase '${key}' array.`);
    }
  }
  const astNodes = (value.astNodes as unknown[]).map(decodeAstNode);
  const targets = (value.targets as unknown[]).map(decodeTarget);
  return { ...value, astNodes, targets } as AuthoringGraph;
}
