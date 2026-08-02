import type { EngineConfig } from './types/domain';

export function modelSourceFingerprint<
  T extends Pick<EngineConfig, 'provider' | 'apiKey' | 'baseUrl'>,
>(engine: T): string {
  return JSON.stringify([engine.provider, engine.apiKey, engine.baseUrl]);
}
