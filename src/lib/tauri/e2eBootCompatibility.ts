import {
  commands,
  type BootProjection,
  type BootRuntimeProjection,
  type Config,
  type ModelCatalogProjection,
  type Result,
  type AppError,
} from './contracts';

function unwrap<T>(result: Result<T, AppError>): T {
  if (result.status === 'ok') return result.data;
  throw result.error;
}

export async function legacyBootProjection(): Promise<BootProjection> {
  const config = unwrap(await commands.getConfig());
  const history = unwrap(await commands.getHistory());
  const last = unwrap(await commands.getLastDesign());
  const pointedThreadId = last?.threadId ?? null;
  const thread = pointedThreadId
    ? history.find((candidate) => candidate.id === pointedThreadId) ?? null
    : history.find((candidate) => !candidate.isBlank) ?? null;
  if (!thread) {
    return { config, history, workspace: null, selectedPartId: null };
  }

  const pointed = pointedThreadId && last?.messageId
    ? unwrap(await commands.getThreadMessageVersion(pointedThreadId, last.messageId))
    : null;
  const selectedVersion = pointed ?? unwrap(await commands.getThreadLatestVersion(thread.id));
  const page = unwrap(await commands.getThreadMessagesPage(thread.id, null, 50, false));
  return {
    config,
    history,
    workspace: {
      thread: { ...thread, messages: [] },
      messagesPage: page ?? {
        messages: selectedVersion ? [selectedVersion] : [],
        nextBefore: null,
        hasMore: false,
        observedBytes: 0,
        truncatedFields: [],
      },
      selectedVersion,
      requestedMessageFound: Boolean(pointed),
    },
    selectedPartId: last?.selectedPartId ?? null,
  };
}

export async function legacyBootRuntimeProjection(): Promise<BootRuntimeProjection> {
  const [config, capabilities] = await Promise.all([
    commands.getConfig().then(unwrap),
    commands.getRuntimeCapabilities().then(unwrap),
  ]);
  return { config, capabilities };
}

export async function legacySaveConfigProjection(config: Config): Promise<BootRuntimeProjection> {
  unwrap(await commands.saveConfig(config));
  const capabilities = unwrap(await commands.getRuntimeCapabilities());
  return { config, capabilities };
}

export async function legacyModelCatalogProjection(): Promise<ModelCatalogProjection> {
  const config = unwrap(await commands.getConfig());
  const engine = config.engines.find((candidate) => candidate.id === config.selectedEngineId);
  if (!engine || (!engine.apiKey && engine.provider !== 'ollama')) {
    return { config, models: [] };
  }
  const models = unwrap(await commands.listModels(engine.provider, engine.apiKey, engine.baseUrl));
  if (models.length > 0 && (!engine.model || !models.includes(engine.model))) {
    engine.model = models[0];
    unwrap(await commands.saveConfig(config));
  }
  return { config, models };
}
