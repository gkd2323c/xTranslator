import { invoke } from "@tauri-apps/api/core";

export interface QueryRequest {
  file_id: string;
  offset: number;
  limit: number;
  filter?: string;
  sort_field?: string;
  sort_dir?: string;
  status_filter?: string;
}

export interface SkyStringDTO {
  id: number;
  source: string;
  translation: string;
  record_sig: string;
  field_sig: string;
  form_id: string;
  status: string;
  list_index: number;
  str_id: number;
}

export interface QueryResponse {
  total: number;
  filtered: number;
  items: SkyStringDTO[];
  offset: number;
  elapsed_ms: number;
}

export interface LoadEspResponse {
  total: number;
  compressed_records: number;
  strings_loaded: number;
  parse_time_ms: number;
  record_counts: Record<string, number>;
  cached: boolean;
}

export interface LoadSstResponse {
  matched: number;
  unmatched: number;
  updated_ids: number[];
  tier_exact: number;
  tier_edid: number;
  tier_normalized: number;
  tier_vocab: number;
  ambiguous: number;
  pending_skipped: number;
  old_data_preserved: number;
  warning: number;
  big_warning: number;
}

export interface HeuristicSearchRequest {
  source: string;
  min_similarity?: number;
  max_results?: number;
}

export interface HeuristicMatchDTO {
  source: string;
  translation: string;
  similarity: number;
  levenshtein: number;
  lcs_len: number;
}

export interface TranslateRequest {
  text: string;
  source_lang?: string;
  target_lang?: string;
  provider?: string;
}

export interface TranslationProvidersResponse {
  current: string;
  available: string[];
  openaiConfigured: boolean;
  deeplConfigured: boolean;
}

export async function queryStrings(request: QueryRequest): Promise<QueryResponse> {
  return invoke("query_strings_command", { request });
}

export async function getStats(): Promise<string> {
  return invoke("get_stats");
}

export async function loadEsp(
  espPath: string,
  stringsDir?: string,
  language?: string,
  game?: string,
): Promise<LoadEspResponse> {
  return invoke("load_esp", { espPath, stringsDir, language, game });
}

export async function loadSst(sstPath: string): Promise<LoadSstResponse> {
  return invoke("load_sst", { sstPath });
}

export async function saveSst(
  sstPath: string,
  masters?: string[],
): Promise<void> {
  return invoke("save_sst", { sstPath, masters });
}

export async function updateTranslation(
  id: number,
  translation: string,
): Promise<void> {
  return invoke("update_translation", { id, translation });
}

export async function heuristicSearch(
  request: HeuristicSearchRequest,
): Promise<HeuristicMatchDTO[]> {
  return invoke("heuristic_search", { request });
}

export async function translateString(
  request: TranslateRequest,
): Promise<string> {
  return invoke("translate_string", { request });
}

export async function setOpenAiApiKey(apiKey: string): Promise<void> {
  await invoke("set_openai_api_key", { apiKey });
  saveConfig({ openai_api_key: apiKey || undefined }).catch(() => {});
}

export async function setDeeplApiKey(apiKey: string): Promise<void> {
  await invoke("set_deepl_api_key", { apiKey });
  saveConfig({ deepl_api_key: apiKey || undefined }).catch(() => {});
}

export async function setTranslationProvider(provider: string): Promise<void> {
  await invoke("set_translation_provider", { provider });
  saveConfig({ current_provider: provider }).catch(() => {});
}

export async function getTranslationProviders(): Promise<TranslationProvidersResponse> {
  const [current, available, openaiConfigured, deeplConfigured] = await invoke<[string, string[], boolean, boolean]>("get_translation_providers");
  return { current, available, openaiConfigured, deeplConfigured };
}

/** @deprecated Use setOpenAiApiKey instead */
export async function setApiKey(apiKey: string): Promise<void> {
  return setOpenAiApiKey(apiKey);
}

export interface XmlExportRequest {
  path: string;
  dest_lang: string;
}

export interface XmlImportResponse {
  matched: number;
  unmatched: number;
  total: number;
  updated_ids: number[];
  tier_exact: number;
  tier_edid: number;
  tier_vocab: number;
  tier_normalized: number;
  ambiguous: number;
  pending_skipped: number;
  old_data_preserved: number;
  warning: number;
  big_warning: number;
}

export async function exportXml(request: XmlExportRequest): Promise<number> {
  return invoke("export_xml", { request });
}

export async function importXml(xmlPath: string): Promise<XmlImportResponse> {
  return invoke("import_xml", { xmlPath });
}

export async function getAllStrings(): Promise<SkyStringDTO[]> {
  return invoke("get_all_strings");
}

export async function getStringsChunk(offset: number, limit: number): Promise<SkyStringDTO[]> {
  return invoke("get_strings_chunk", { offset, limit });
}

export async function getStringsCount(): Promise<number> {
  return invoke("get_strings_count");
}

export async function getIsDirty(): Promise<boolean> {
  return invoke("get_is_dirty");
}

export interface SaveStringsRequest {
  output_dir: string;
  target_lang: string;
  base_name: string;
}

export interface SaveStringsResponse {
  strings_count: number;
  dlstrings_count: number;
  ilstrings_count: number;
  translated_count: number;
}

export async function saveStrings(request: SaveStringsRequest): Promise<SaveStringsResponse> {
  return invoke("save_strings", { request });
}

// ── Batch Processor Types ──────────────────────────────────────────

export interface BatchEntry {
  esp_path: string;
  strings_dir?: string;
  language?: string;
  game?: string;
  sst_path?: string;
}

export interface BatchConfig {
  entries: BatchEntry[];
  provider?: string;
  target_lang?: string;
  skip_translated?: boolean;
}

export interface BatchStatus {
  job_id: string;
  job_type: string;
  total_files: number;
  completed_files: number;
  failed_files: number;
  current_file?: string;
  current_file_progress: number;
  total_strings: number;
  translated_strings: number;
  is_running: boolean;
  is_cancelled: boolean;
  is_completed: boolean;
  is_failed: boolean;
  errors: string[];
  elapsed_ms: number;
}

export interface BatchProgress {
  job_id: string;
  file_path: string;
  stage: string;
  current_file: number;
  total_files: number;
  strings_translated: number;
  total_strings: number;
  message: string;
}

export interface BatchFileComplete {
  job_id: string;
  file_path: string;
  translated: number;
  skipped: number;
  errors: number;
  duration_ms: number;
}

export interface BatchFileError {
  file_path: string;
  message: string;
}

export interface BatchComplete {
  job_id: string;
  total_files: number;
  success: number;
  failed: number;
  total_translated: number;
  total_errors: number;
  duration_ms: number;
  is_cancelled: boolean;
  errors: BatchFileError[];
}

// ── Batch Processor API Wrappers ───────────────────────────────────

export async function startBatchTranslate(config: BatchConfig): Promise<string> {
  return invoke("start_batch_translate", { config });
}

export async function startBatchExport(
  entries: BatchEntry[],
  outputDir: string,
  exportFormat: string,
): Promise<string> {
  return invoke("start_batch_export", { entries, outputDir, exportFormat });
}

export async function getBatchStatus(): Promise<BatchStatus | null> {
  return invoke("get_batch_status");
}

export async function cancelBatchJob(): Promise<void> {
  return invoke("cancel_batch_job");
}

export async function listEspFiles(dir: string): Promise<string[]> {
  return invoke("list_esp_files", { dir });
}

// ── Auto Backup Types ──────────────────────────────────────────────

export interface AutoBackupRequest {
  sst_path: string;
  max_backups?: number;
}

export interface AutoBackupResponse {
  backup_path: string | null;
  total_backups: number;
}

export async function autoBackupSst(request: AutoBackupRequest): Promise<AutoBackupResponse> {
  return invoke("auto_backup_sst", { request });
}

// ── BSA Browser Types ──────────────────────────────────────────────

export interface BsaFileEntryDto {
  path: string;
  size: number;
  compressed: boolean;
  folder: string;
}

export interface BsaFileListDto {
  archive_name: string;
  version: number;
  total_files: number;
  folders: string[];
  files: BsaFileEntryDto[];
}

export async function listBsaFiles(bsaPath: string): Promise<BsaFileListDto> {
  return invoke("list_bsa_files", { bsaPath });
}

export async function listBa2Files(ba2Path: string): Promise<BsaFileListDto> {
  return invoke("list_ba2_files", { ba2Path });
}

export async function extractBsaFile(bsaPath: string, filePath: string, outputDir: string): Promise<string> {
  return invoke("extract_bsa_file", { bsaPath, filePath, outputDir });
}

export async function extractBa2File(ba2Path: string, filePath: string, outputDir: string): Promise<string> {
  return invoke("extract_ba2_file", { ba2Path, filePath, outputDir });
}

export async function extractBsaFolder(bsaPath: string, folder: string, outputDir: string): Promise<string[]> {
  return invoke("extract_bsa_folder", { bsaPath, folder, outputDir });
}

export async function extractBa2Folder(ba2Path: string, folder: string, outputDir: string): Promise<string[]> {
  return invoke("extract_ba2_folder", { ba2Path, folder, outputDir });
}

// ── PEX Types ───────────────────────────────────────────────────────

export interface PexTranslatableDto {
  object_name: string;
  state_name: string;
  function_name: string;
  string_type: string;
  source_text: string;
}

export interface PexScriptDto {
  script_name: string;
  game_id: number;
  major_version: number;
  minor_version: number;
  string_count: number;
  translatable: PexTranslatableDto[];
}

export async function parsePexStrings(pexPath: string): Promise<PexScriptDto> {
  return invoke("parse_pex_strings", { pexPath });
}

// ── FUZ Types ───────────────────────────────────────────────────────

export interface FuzMapping {
  response_id: number;
  dialog_text: string;
  fuz_file: string;
  duration_secs: number;
}

export interface FuzScanResponse {
  fuz_mappings: FuzMapping[];
  total_fuz_files: number;
}

export async function scanFuzDirectory(voiceDir: string): Promise<FuzScanResponse> {
  return invoke("scan_fuz_directory", { voiceDir });
}

export async function getFuzAudioData(fuzPath: string): Promise<number[]> {
  return invoke("get_fuz_audio_data", { fuzPath });
}

// ── Dialog Tree Types ───────────────────────────────────────────────

export interface DialogInfoDto {
  id: number;
  form_id: number;
  source: string;
  translation: string;
  dialog_text: string;
}

export interface NpcDialogDto {
  npc_edid: string;
  dialogues: DialogInfoDto[];
}

export interface DialogTreeDto {
  npcs: NpcDialogDto[];
}

export async function buildDialogTree(): Promise<DialogTreeDto> {
  return invoke("build_dialog_tree");
}

// ── ESP Compare Types ───────────────────────────────────────────────

export interface EspComparePairDto {
  new_id: number;
  old_id: number;
  source: string;
  record_sig: string;
  field_sig: string;
  old_source: string;
  new_source: string;
}

export interface EspCompareResultDto {
  identical_count: number;
  added_count: number;
  removed_count: number;
  modified_count: number;
  identical: EspComparePairDto[];
  added: EspComparePairDto[];
  removed: EspComparePairDto[];
  modified: EspComparePairDto[];
}

export async function compareEspFiles(
  oldEspPath: string,
  newEspPath: string,
  dataDir?: string,
  game?: string,
): Promise<EspCompareResultDto> {
  return invoke("compare_esp_files", {
    oldEspPath,
    newEspPath,
    dataDir,
    game,
  });
}

// ── MCM Types ───────────────────────────────────────────────────────

export interface McmEntryDto {
  id: string;
  source: string;
  translation: string;
  line_index: number;
  byte_offset: number;
}

export interface McmFileDto {
  path: string;
  entry_count: number;
  encoding: string;
  entries: McmEntryDto[];
}

export interface McmSaveRequest {
  path: string;
  entries: McmEntryDto[];
}

// ── Config ──────────────────────────────────────────────────────────

export interface AppConfigDto {
  openai_api_key?: string;
  deepl_api_key?: string;
  current_provider?: string;
  theme?: string;
  language?: string;
  proxy_server?: string;
  proxy_port?: number;
  proxy_username?: string;
  proxy_password?: string;
}

// ── API Config ──────────────────────────────────────────────────────

export interface ApiProviderInfo {
  name: string;
  label: string;
  enabled: boolean;
  models: string[];
  default_query: string | null;
  char_limit: number;
  array_limit: number;
}

export interface ApiConfigResponse {
  providers: ApiProviderInfo[];
}

export async function loadConfig(): Promise<AppConfigDto> {
  return invoke("load_config");
}

export async function saveConfig(config: AppConfigDto): Promise<void> {
  return invoke("save_config", { config });
}

export async function getApiConfig(): Promise<ApiConfigResponse> {
  return invoke("get_api_config");
}

export async function loadMcmFile(mcmPath: string): Promise<McmFileDto> {
  return invoke("load_mcm_file", { mcmPath });
}

export async function saveMcmFile(request: McmSaveRequest): Promise<void> {
  return invoke("save_mcm_file", { request });
}
