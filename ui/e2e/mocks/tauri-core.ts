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

    case "start_string_batch_translate":
      return { batch_id: "mock-batch-strings-001" } as T;

    case "cancel_string_batch_translate":
    case "cancel_batch_translate":
    case "clear_cache":
      return undefined as T;

    case "get_batch_status":
      return {
        status: "running",
        entries: [
          { path: "mock1.esp", status: "pending" as const },
          { path: "mock2.esp", status: "running" as const },
          { path: "mock3.esp", status: "done" as const, translated: 10, total: 15 },
        ],
        total: 3,
        done: 1,
        failed: 0,
        total_translated: 10,
        total_strings: 50,
      } as T;

    case "spell_check_load":
      return { available_dictionaries: ["en_US"], current_dictionary: "en_US", active: true, loaded: true } as T;

    case "spell_check_unload":
      return undefined as T;

    case "spell_check_toggle":
      return false as T;

    case "spell_check_config":
      return { available_dictionaries: ["en_US", "en_GB"], current_dictionary: "en_US", active: true, loaded: true } as T;

    case "spell_check_text":
      return {
        has_faults: true,
        total_words: 5,
        fault_count: 1,
        faults: [
          { word: "adventurer", start_byte: 10, end_byte: 20, suggestions: ["adventurer", "adventures"] },
        ],
        fault_ratio_locked: false,
      } as T;

    case "spell_check_suggestions":
      return ["adventurer", "adventures", "adventure"] as T;

    case "spell_check_ignore":
      return undefined as T;

    // ── MCM ──────────────────────────────────────────────────────
    case "load_mcm_file": {
      const mcmEntries = [
        { id: "$sGeneral", source: "General Settings", translation: "常规设置", line_index: 0, byte_offset: 0 },
        { id: "$sAudio", source: "Audio", translation: "", line_index: 1, byte_offset: 30 },
        { id: "$sVideo", source: "Video", translation: "视频", line_index: 2, byte_offset: 50 },
        { id: "$sLanguage", source: "Language", translation: "语言", line_index: 3, byte_offset: 70 },
        { id: "$sSubtitles", source: "Subtitles", translation: "", line_index: 4, byte_offset: 90 },
        { id: "$sDifficulty", source: "Difficulty", translation: "", line_index: 5, byte_offset: 115 },
      ];
      return {
        path: args?.mcmPath as string ?? "C:/mock/Settings.txt",
        entry_count: mcmEntries.length,
        encoding: "UTF-16LE",
        entries: mcmEntries,
      } as T;
    }

    case "save_mcm_file":
      return undefined as T;

    case "mcm_compare": {
      const req = args?.request as { entries?: Array<{ id: string; translation: string; line_index: number; source: string; byte_offset: number }>; policy?: string } | undefined;
      const updated = (req?.entries ?? []).filter((e) => e.translation === "").map((e) => ({
        ...e,
        translation: `[ref] ${e.source}`,
      }));
      return {
        matched: (req?.entries ?? []).length,
        unmatched: 0,
        updated_entries: updated,
      } as T;
    }

    // ── ESP Compare ──────────────────────────────────────────────
    case "compare_esp_files": {
      const mockPairs = [
        { new_id: 0x100, old_id: 0x100, source: "Hello", record_sig: "INFO", field_sig: "FULL", old_source: "Hello", new_source: "Hello" },
        { new_id: 0x101, old_id: 0x101, source: "Guard text", record_sig: "INFO", field_sig: "FULL", old_source: "Guard text", new_source: "Guard dialogue" },
        { new_id: 0x102, old_id: 0, source: "New dialogue", record_sig: "QUST", field_sig: "FULL", old_source: "", new_source: "New dialogue" },
        { new_id: 0, old_id: 0x103, source: "Removed text", record_sig: "INFO", field_sig: "FULL", old_source: "Removed text", new_source: "" },
      ];
      return {
        identical_count: 1,
        added_count: 1,
        removed_count: 1,
        modified_count: 1,
        identical: [mockPairs[0]],
        added: [mockPairs[2]],
        removed: [mockPairs[3]],
        modified: [mockPairs[1]],
      } as T;
    }

    // ── FUZ ──────────────────────────────────────────────────────
    case "scan_fuz_directory": {
      const fuzMappings = [
        { response_id: 0x100, dialog_text: "Hello, I am a guard.", fuz_file: "D:/Voice/guard_1.fuz", duration_secs: 2.5, has_lip: true, parse_ok: true },
        { response_id: 0x101, dialog_text: "I used to be an adventurer.", fuz_file: "D:/Voice/adventurer.fuz", duration_secs: 3.2, has_lip: true, parse_ok: true },
        { response_id: 0x102, dialog_text: "Wait, I know you.", fuz_file: "D:/Voice/wait.fuz", duration_secs: 1.8, has_lip: false, parse_ok: true },
        { response_id: 0x103, dialog_text: "", fuz_file: "D:/Voice/broken.fuz", duration_secs: 0, has_lip: false, parse_ok: false },
      ];
      return {
        fuz_mappings: fuzMappings,
        total_fuz_files: 10,
      } as T;
    }

    case "get_fuz_audio_data":
      // Return a minimal WAV header with silence (44 bytes header + 1 sec of silence at 8kHz)
      return Array.from({ length: 8044 }, (_, i) => i < 44 ? [0x52, 0x49, 0x46, 0x46, 0x24, 0x1F, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6D, 0x74, 0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x40, 0x1F, 0x00, 0x00, 0x40, 0x1F, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x64, 0x61, 0x74, 0x61, 0x00, 0x1F, 0x00, 0x00][i] || 0 : 128) as T;

    // ── SST Merge ────────────────────────────────────────────────
    case "sst_merge": {
      const req2 = args?.request as { source_path?: string; policy?: string } | undefined;
      return {
        added: req2?.policy !== "skip_new" ? 5 : 0,
        updated: req2?.policy !== "skip_update" ? 3 : 0,
        overwritten: req2?.policy === "overwrite" ? 2 : 0,
        skipped: req2?.policy === "overwrite" ? 0 : 8,
        total_source: 50,
        total_target: 128,
      } as T;
    }

    // ── BSA ──────────────────────────────────────────────────────
    case "list_bsa_files":
      return {
        files: [
          { path: "meshes/armor/iron/iron_helmet.nif", size: 45000, uncompressed_size: 62000, compressed: true },
          { path: "textures/armor/iron/iron_helmet.dds", size: 280000, uncompressed_size: 280000, compressed: false },
          { path: "meshes/weapons/iron/iron_sword.nif", size: 32000, uncompressed_size: 44000, compressed: true },
          { path: "music/exploration/music_x_1.mp3", size: 1200000, uncompressed_size: 1200000, compressed: false },
        ],
        total_files: 4,
        total_size: 1557000,
      } as T;

    case "extract_bsa_file":
    case "extract_bsa_folder":
      return undefined as T;

    // ── PEX ──────────────────────────────────────────────────────
    case "parse_pex_strings": {
      return [
        {
          script_name: "QF_MQ101_000337B5",
          game_id: 1,
          versions: [1],
          translatable: [
            { source: "Wait, I know you", translation: "", property_name: "pDialogue", function_name: "Fragment_0", instruction_offset: 12 },
            { source: "Let me guess...", translation: "", property_name: "pDialogue", function_name: "Fragment_1", instruction_offset: 24 },
          ],
        },
      ] as T;
    }

    case "compile_pex":
      return "/mock/output/test.pex" as T;

    // ── Config (full) ─────────────────────────────────────────────
    case "load_config":
      return {
        theme: "dark",
        language: "en",
        current_provider: "openai",
        openai_api_key: "",
        deepl_api_key: "",
        spellcheck_dictionary: "en_US",
        spellcheck_active: true,
        spellcheck_loaded: true,
        proxy_server: "",
        proxy_port: 0,
        esp_mode: false,
      } as T;

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

// Expose for Playwright test access via page.evaluate()
if (typeof window !== "undefined") {
  (window as any).__setMockResult = __setMockResult;
  (window as any).__clearMockResults = __clearMockResults;
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
