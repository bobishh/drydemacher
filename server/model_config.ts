export type ModelProvider = 'gemini' | 'ollama' | 'openai';

export interface ModelRequestConfig {
  provider: ModelProvider;
  apiKey: string;
  model: string;
  baseUrl: string;
}

type ModelEnvironment = Readonly<Record<string, string | undefined>>;

function readString(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim() ? value.trim() : undefined;
}

export function resolveModelConfig(
  body: Record<string, unknown>,
  environment: ModelEnvironment = process.env,
): ModelRequestConfig {
  if (
    Object.prototype.hasOwnProperty.call(body, 'api_key') ||
    Object.prototype.hasOwnProperty.call(body, 'base_url')
  ) {
    throw new Error('Model config fields must use camelCase: apiKey, baseUrl.');
  }

  const requestedProvider = readString(body.provider) ?? environment.MODEL_PROVIDER ?? 'gemini';
  if (
    requestedProvider !== 'gemini' &&
    requestedProvider !== 'ollama' &&
    requestedProvider !== 'openai'
  ) {
    throw new Error(`Unsupported provider: ${requestedProvider}`);
  }

  const requestApiKey = readString(body.apiKey);
  const requestBaseUrl = readString(body.baseUrl);
  if (requestBaseUrl && !requestApiKey) {
    throw new Error('Request baseUrl requires request apiKey.');
  }

  const apiKeyEnvironment =
    requestedProvider === 'gemini' ? 'GEMINI_API_KEY' : 'OPENAI_API_KEY';
  const defaultModel = requestedProvider === 'gemini' ? 'gemini-2.5-flash' : 'gpt-4o';
  return {
    provider: requestedProvider,
    apiKey:
      requestApiKey ??
      environment.MODEL_API_KEY ??
      environment[apiKeyEnvironment] ??
      '',
    model: readString(body.model) ?? environment.MODEL_NAME ?? defaultModel,
    baseUrl: requestApiKey ? (requestBaseUrl ?? '') : (environment.MODEL_BASE_URL ?? ''),
  };
}
