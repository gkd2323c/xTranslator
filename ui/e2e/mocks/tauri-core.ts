/**
 * Mock for @tauri-apps/api/core
 * Used in Playwright E2E tests when the Rust backend is not available.
 * Returns realistic mock data for the xTranslator translation tool.
 */

// Centralized mock data store - shared across all mock invoke handlers
const mockData: Record<string, unknown> = {};

export async function invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  // Allow tests to pre-seed mock data
  const seedKey = `__invoke_result:${cmd}`;
  if (mockData[seedKey] !== undefined) {
    return mockData[seedKey] as T;
  }

  switch (cmd) {
    case "get_stats":
      return '{"total":128,"translated":45,"incomplete":83}' as T;

    case "query_strings_command":
      return {
        total: 128,
        filtered: 128,
        items: generateMockStrings(args?.request as { offset?: number; limit?: number }),
        offset: (args?.request as { offset?: number })?.offset ?? 0,
        elapsed_ms: 12,
      } as T;

    case "get_all_strings":
      return generateMockStrings({}) as T;

    case "get_strings_count":
      return 128 as T;

    case "get_is_dirty":
      return false as T;

    case "get_strings_chunk": {
      const offset = (args?.offset as number) ?? 0;
      const limit = (args?.limit as number) ?? 50;
      return generateMockStrings({ offset, limit }) as T;
    }

    case "load_sst":
      return {
        matched: 0,
        unmatched: 128,
        updated_ids: [],
        tier_exact: 0,
        tier_edid: 0,
        tier_normalized: 0,
        tier_vocab: 0,
        ambiguous: 0,
        pending_skipped: 0,
        old_data_preserved: 0,
        warning: 0,
        big_warning: 0,
      } as T;

    case "load_esp":
      return {
        total: 128,
        compressed_records: 0,
        strings_loaded: 128,
        parse_time_ms: 45,
        record_counts: { INFO: 80, QUST: 30, DIAL: 18 },
        cached: false,
        esp_hash: "mock_hash_abc123",
      } as T;

    case "save_sst":
    case "update_translation":
    case "batch_update_translations":
      return undefined as T;

    case "heuristic_search":
      return [
        { source: "Hello", translation: "你好", similarity: 0.9, levenshtein: 0, lcs_len: 5 },
        { source: "Hello world", translation: "你好世界", similarity: 0.8, levenshtein: 1, lcs_len: 5 },
      ] as T;

    case "translate_string":
      return { translated: `[翻译] ${(args?.request as { text?: string })?.text ?? ""}` } as T;

    case "get_translation_providers":
      return {
        current: "openai",
        available: ["openai", "deepl", "baidu", "youdao", "azure", "google"],
        openaiConfigured: true,
        deeplConfigured: false,
        baiduConfigured: false,
        youdaoConfigured: false,
        azureConfigured: false,
        googleConfigured: false,
      } as T;

    case "set_openai_api_key":
    case "set_deepl_api_key":
    case "set_baidu_api_key":
    case "set_yooudao_api_key":
    case "set_azure_api_key":
    case "set_translation_provider":
      return undefined as T;

    case "save_config":
      return undefined as T;

    case "save_strings":
      return { saved: 10 } as T;

    case "export_xml":
      return { path: "/mock/output.xml", count: 128 } as T;

    case "import_xml":
      return { matched: 30, new_items: 10, errors: [] } as T;

    case "toolbox_transform":
      return { ids: [], error_count: 0 } as T;

    case "start_batch_translate":
      return { batch_id: "mock-batch-001" } as T;

    case "cancel_batch_translate":
    case "clear_cache":
      return undefined as T;

    case "config_get":
      return args?.key === "data_configs" ? [] : {} as T;

    case "get_data_configs":
      return [] as T;

    case "save_data_config":
      return undefined as T;

    default:
      console.warn(`[Tauri Mock] Unhandled invoke command: ${cmd}`, args);
      return undefined as T;
  }
}

/** Allow tests to override specific invoke results */
export function __setMockResult(cmd: string, data: unknown): void {
  mockData[`__invoke_result:${cmd}`] = data;
}

/** Clear all mock result overrides */
export function __clearMockResults(): void {
  Object.keys(mockData).forEach((key) => delete mockData[key]);
}

// ---- Helpers ----

function generateMockStrings(opts: { offset?: number; limit?: number }): Array<{
  id: number;
  source: string;
  translation: string;
  record_sig: string;
  field_sig: string;
  form_id: string;
  status: string;
  list_index: number;
  str_id: number;
  is_vmad: boolean;
  ld: number;
}> {
  const offset = opts.offset ?? 0;
  const limit = opts.limit ?? 50;
  const items: Array<{
    id: number;
    source: string;
    translation: string;
    record_sig: string;
    field_sig: string;
    form_id: string;
    status: string;
    list_index: number;
    str_id: number;
    is_vmad: boolean;
    ld: number;
  }> = [];

  const sampleSources = [
    "Hello, I am a guard.",
    "I used to be an adventurer like you.",
    "Let me guess... someone stole your sweetroll?",
    "Need something?",
    "Be seeing you.",
    "Wait, I know you.",
    "Can't wait for the next shipment.",
    "Another settlement needs our help.",
    "I don't have time for this.",
    "Stay a while and listen.",
    "What is it now?",
    "You're finally awake.",
    "HALT! You have violated the law.",
    "Never should have come here.",
    "By the Nine Divines, watch your tongue.",
  ];

  for (let i = 0; i < limit; i++) {
    const idx = offset + i;
    if (idx >= 128) break;
    const source = sampleSources[idx % sampleSources.length];
    items.push({
      id: idx,
      source,
      translation: idx < 45 ? `[已翻译] ${source}` : "",
      record_sig: ["INFO", "QUST", "DIAL"][idx % 3],
      field_sig: "FULL",
      form_id: `0x${(0x10000000 + idx).toString(16).toUpperCase()}`,
      status: idx < 45 ? "translated" : "incomplete",
      list_index: idx,
      str_id: 1000 + idx,
      is_vmad: false,
      ld: idx < 45 ? 0 : -1,
    });
  }
  return items;
}
