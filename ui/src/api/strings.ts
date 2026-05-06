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
  is_vmad: boolean;
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
  esp_hash: string;
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
  baiduConfigured: boolean;
  youdaoConfigured: boolean;
  azureConfigured: boolean;
  googleConfigured: boolean;
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

export async function batchUpdateTranslations(
  updates: [number, string][],
): Promise<number> {
  return invoke("batch_update_translations", { updates });
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

export async function setBaiduApiKey(appId: string, key: string): Promise<void> {
  await invoke("set_baidu_api_key", { appId, key });
  saveConfig({ baidu_app_id: appId || undefined, baidu_key: key || undefined }).catch(() => {});
}

export async function setYoudaoApiKey(appKey: string, secretKey: string): Promise<void> {
  await invoke("set_yooudao_api_key", { appKey, secretKey });
  saveConfig({ youdao_app_key: appKey || undefined, youdao_secret_key: secretKey || undefined }).catch(() => {});
}

export async function setAzureApiKey(apiKey: string): Promise<void> {
  await invoke("set_azure_api_key", { apiKey });
  saveConfig({ azure_key: apiKey || undefined }).catch(() => {});
}

export async function toolboxTransform(
  tool: string,
  target: string,
  ids: number[],
  headerText?: string,
): Promise<number> {
  return invoke("toolbox_transform", { tool, target, ids, headerText });
}

// ── Spell Check ────────────────────────────────────────────────────

export interface SpellFaultDto {
  word: string;
  start_byte: number;
  end_byte: number;
}

export interface SpellCheckResultDto {
  faults: SpellFaultDto[];
  total_words: number;
  fault_ratio_locked: boolean;
  active: boolean;
}

export interface SpellCheckConfigDto {
  available_dictionaries: string[];
  current_dictionary: string | null;
  active: boolean;
  loaded: boolean;
}

export async function spellCheckLoad(dllPath: string, dictDir: string, dictName: string): Promise<SpellCheckConfigDto> {
  return invoke("spell_check_load", { dllPath, dictDir, dictName });
}

export async function spellCheckUnload(): Promise<void> {
  return invoke("spell_check_unload");
}

export async function spellCheckToggle(): Promise<boolean> {
  return invoke("spell_check_toggle");
}

export async function spellCheckConfig(dictDir: string): Promise<SpellCheckConfigDto> {
  return invoke("spell_check_config", { dictDir });
}

export async function spellCheckText(text: string): Promise<SpellCheckResultDto> {
  return invoke("spell_check_text", { text });
}

export async function spellCheckSuggestions(word: string): Promise<string[]> {
  return invoke("spell_check_suggestions", { word });
}

export async function spellCheckIgnore(word: string, ignorePath: string): Promise<void> {
  return invoke("spell_check_ignore", { word, ignorePath });
}

// ── Header Processor ──────────────────────────────────────────────

export interface HeaderRuleDto {
  index: number;
  header: string;
  r_sig: string;
  f_sig: string;
  enabled: boolean;
  in_edid: string[];
  ex_edid: string[];
  no_kw: boolean;
  any_kw: boolean;
  has_component: boolean;
  include_keywords: HeaderKeywordDto[];
  pre_process: boolean;
  full_replace: boolean;
  include_or: boolean;
  is_fallback: boolean;
  regex: string | null;
  tag_id: number;
}

export interface HeaderKeywordDto {
  kw_type: string;
  name: string;
  form_id: number;
}

export interface HeaderApplyResult {
  total_rules: number;
  enabled_rules: number;
  strings_matched: number;
}

export async function headerRulesLoad(path: string): Promise<HeaderRuleDto[]> {
  return invoke("header_rules_load", { path });
}

export async function headerRulesList(): Promise<HeaderRuleDto[]> {
  return invoke("header_rules_list");
}

export async function headerRulesToggle(index: number, enabled: boolean): Promise<void> {
  return invoke("header_rules_toggle", { index, enabled });
}

export async function headerRulesApply(): Promise<HeaderApplyResult> {
  return invoke("header_rules_apply");
}

export async function headerRulesSave(path: string): Promise<void> {
  return invoke("header_rules_save", { path });
}

export async function setTranslationProvider(provider: string): Promise<void> {
  await invoke("set_translation_provider", { provider });
  saveConfig({ current_provider: provider }).catch(() => {});
}

export async function getTranslationProviders(): Promise<TranslationProvidersResponse> {
  const [current, available, openaiConfigured, deeplConfigured, baiduConfigured, youdaoConfigured, azureConfigured, googleConfigured] = await invoke<[string, string[], boolean, boolean, boolean, boolean, boolean, boolean]>("get_translation_providers");
  return { current, available, openaiConfigured, deeplConfigured, baiduConfigured, youdaoConfigured, azureConfigured, googleConfigured };
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

export async function parsePexStrings(pexPath: string, game?: string): Promise<PexScriptDto> {
  return invoke("parse_pex_strings", { pexPath, game });
}

export interface DecompilePexResponse {
  script_name: string;
  object_count: number;
  function_count: number;
  instruction_count: number;
  pseudocode: string;
}

export async function decompilePex(pexPath: string): Promise<DecompilePexResponse> {
  return invoke("decompile_pex", { pexPath });
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
  baidu_app_id?: string;
  baidu_key?: string;
  youdao_app_key?: string;
  youdao_secret_key?: string;
  azure_key?: string;
  current_provider?: string;
  theme?: string;
  language?: string;
  proxy_server?: string;
  proxy_port?: number;
  proxy_username?: string;
  proxy_password?: string;
  esp_mode?: boolean;
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

export type McmComparePolicy = "all" | "no_trans" | "no_trans_and_partial" | "partial_only";

export interface McmCompareRequest {
  entries: McmEntryDto[];
  reference_path: string;
  policy: McmComparePolicy;
}

export interface McmCompareResult {
  matched: number;
  unmatched: number;
  updated_entries: McmEntryDto[];
}

export async function mcmCompare(request: McmCompareRequest): Promise<McmCompareResult> {
  return invoke("mcm_compare", { request });
}

// ── TCSC ─────────────────────────────────────────────────────────────

export async function tcscConvert(text: string, direction: "to_simplified" | "to_traditional"): Promise<string> {
  return invoke("tcsc_convert", { text, direction });
}

export async function tcscBatchConvert(direction: "to_simplified" | "to_traditional", ids?: number[]): Promise<number[]> {
  return invoke("tcsc_batch_convert", { direction, ids });
}

export async function rtlReverse(text: string): Promise<string> {
  return invoke("rtl_reverse", { text });
}

export async function shapeArabic(text: string): Promise<string> {
  return invoke("shape_arabic", { text });
}

export async function deshapeArabic(text: string): Promise<string> {
  return invoke("deshape_arabic", { text });
}

// ── Vocabulary Types ────────────────────────────────────────────────

export interface VocabularyInfo {
  pair_count: number;
  base_names: string[];
}

export async function loadVocabulary(
  stringsDir: string,
  sourceLang: string,
  targetLang: string,
  game?: string,
): Promise<VocabularyInfo> {
  return invoke("load_vocabulary", { stringsDir, sourceLang, targetLang, game });
}

// ── Finalize Types ──────────────────────────────────────────────────

export interface FinalizeRequest {
  strings_output_dir: string;
  target_lang: string;
  base_name: string;
  sst_path?: string;
  xml_path?: string;
}

export interface FinalizeResponse {
  strings_path: string;
  dlstrings_path: string;
  ilstrings_path: string;
  sst_path: string;
  xml_path: string;
  translated_count: number;
  total_count: number;
}

export async function finalize(request: FinalizeRequest): Promise<FinalizeResponse> {
  return invoke("finalize", { request });
}

// ── Source/Dest Compare Types ────────────────────────────────────────

export async function compareSourceDest(mode: "diff" | "same"): Promise<number> {
  return invoke("compare_source_dest", { mode });
}

// ── Alias Check Types ────────────────────────────────────────────────

export interface AliasCheckResult {
  source_aliases: string[];
  trans_aliases: string[];
  missing_in_trans: string[];
  extra_in_trans: string[];
  has_mismatch: boolean;
}

export async function checkAliases(id: number): Promise<AliasCheckResult> {
  return invoke("check_aliases", { id });
}

// ── Data Config Types ────────────────────────────────────────────────

export interface CtdaFuncDto {
  id: number;
  name: string;
  params: string;
}

export interface FieldSizeInfoDto {
  max_size: number;
  can_wrap: boolean;
}

export interface DataConfigsDto {
  ctda_funcs: CtdaFuncDto[];
  field_size_ref: Record<string, FieldSizeInfoDto>;
  dial_sub_type: Record<string, string>;
  emote_definition: Record<string, string>;
}

export async function loadDataConfigs(game: string): Promise<DataConfigsDto> {
  return invoke("load_data_configs", { game });
}

// ── ESP Write-back Types ─────────────────────────────────────────────

export interface SaveEspRequest {
  path: string;
  create_backup: boolean;
}

export interface SaveEspResponse {
  bytes_written: number;
  records_modified: number;
}

export interface FinalizeEspRequest {
  esp_path: string;
  strings_dir: string;
  base_name: string;
  language: string;
  create_backup: boolean;
}

export interface FinalizeEspResponse {
  esp_path: string;
  strings_files: string[];
  records_modified: number;
}

export interface DelocalizeEspRequest {
  esp_path: string;
  strings_dir: string;
  base_name: string;
  language: string;
  create_backup: boolean;
}

export interface DelocalizeEspResponse {
  new_string_count: number;
  strings_files_paths: string[];
}

export interface EspHeaderInfoDto {
  version: number;
  num_records: number;
  next_object_id: number;
  author: string;
  description: string;
  masters: string[];
  overridden_count: number;
  is_master: boolean;
  is_localized: boolean;
}

export async function saveEsp(request: SaveEspRequest): Promise<SaveEspResponse> {
  return invoke("save_esp", { request });
}

export async function getEspHeader(): Promise<EspHeaderInfoDto> {
  return invoke("get_esp_header");
}

export async function finalizeEsp(request: FinalizeEspRequest): Promise<FinalizeEspResponse> {
  return invoke("finalize_esp", { request });
}

export async function delocalizeEsp(request: DelocalizeEspRequest): Promise<DelocalizeEspResponse> {
  return invoke("delocalize_esp", { request });
}

export interface RecoveryInfo {
  esp_name: string;
  pending_count: number;
  cache_file_path: string;
}

export interface CheckPendingCacheResponse {
  recovery: RecoveryInfo | null;
}

export interface ApplyCacheResponse {
  applied_count: number;
}

export async function checkPendingCache(espHash: string): Promise<CheckPendingCacheResponse> {
  return invoke("check_pending_cache", { espHash });
}

export async function applyTranslationCache(espHash: string): Promise<ApplyCacheResponse> {
  return invoke("apply_translation_cache", { espHash });
}

export async function discardTranslationCache(espHash: string): Promise<void> {
  return invoke("discard_translation_cache", { espHash });
}

export async function startStringBatchTranslate(ids: number[], concurrency: number): Promise<string> {
  return invoke("start_string_batch_translate", { ids, concurrency });
}

export async function cancelStringBatchTranslate(): Promise<void> {
  return invoke("cancel_string_batch_translate");
}
