import { invoke } from "@tauri-apps/api/core";

// 虚拟滚动分页查询请求
export interface QueryRequest {
  // 文件 ID（当前固定为 "test"）
  file_id: string;
  // 视口起始偏移（0-based）
  offset: number;
  // 视口大小（通常 50-100）
  limit: number;
  // 搜索过滤词（在 source/translation 中搜索）
  filter?: string;
  // 排序字段（如 "source", "translation", "form_id"）
  sort_field?: string;
  // 排序方向："asc" 或 "desc"
  sort_dir?: string;
  // 状态筛选："translated" / "incomplete" / "locked"
  status_filter?: string;
}

// 前端展示的字符串 DTO
export interface SkyStringDTO {
  // 内部稳定 ID（用于前端定位）
  id: number;
  // 源文本（原文）
  source: string;
  // 翻译文本（译文）
  translation: string;
  // 记录类型签名（如 "INFO", "DIAL"）
  record_sig: string;
  // 字段签名（如 "FULL", "DESC"）
  field_sig: string;
  // FormID（十六进制字符串，如 "0x00012345"）
  form_id: string;
  // 翻译状态："translated" / "incomplete" / "locked"
  status: string;
  // Strings 文件类型索引：0=.STRINGS, 1=.DLSTRINGS, 2=.ILSTRINGS
  list_index: number;
  // Strings 文件中的字符串 ID
  str_id: number;
  // 是否为 VMAD 脚本字符串
  is_vmad: boolean;
  // 启发式搜索匹配数量（0-255）
  ld: number;
}

// 虚拟滚动分页查询响应
export interface QueryResponse {
  // 总记录数（未过滤）
  total: number;
  // 过滤后的记录数
  filtered: number;
  // 当前视口数据
  items: SkyStringDTO[];
  // 当前偏移（回显请求中的 offset）
  offset: number;
  // 响应耗时（毫秒）
  elapsed_ms: number;
}

// ESP 加载响应
export interface LoadEspResponse {
  // 解析出的总字符串数
  total: number;
  // 压缩记录数
  compressed_records: number;
  // 成功加载的 Strings 文件数 (0-3)
  strings_loaded: number;
  // 解析耗时（毫秒）；缓存命中时为 0
  parse_time_ms: number;
  // 各记录类型数量统计
  record_counts: Record<string, number>;
  // 是否从缓存加载
  cached: boolean;
  // ESP 文件 SHA-256 哈希
  esp_hash: string;
}

// SST 加载响应
export interface LoadSstResponse {
  // 匹配成功的条目数
  matched: number;
  // 未匹配的 SST 条目数
  unmatched: number;
  // 被更新的字符串 ID 列表
  updated_ids: number[];
  // Tier 1 精确三元组匹配数
  tier_exact: number;
  // Tier 2 EDID 哈希匹配数
  tier_edid: number;
  // Tier 3 规范化文本匹配数
  tier_normalized: number;
  // Tier 4 词汇重叠匹配数
  tier_vocab: number;
  // 歧义但未自动应用的条目数
  ambiguous: number;
  // 因 pending 状态跳过的条目数
  pending_skipped: number;
  // 保留为 oldData 的条目数
  old_data_preserved: number;
  // 因 index/indexMax 可疑而标记 warning 的条目数
  warning: number;
  // 因 index/indexMax 不一致而标记 bigWarning 的条目数
  big_warning: number;
}

// 启发式搜索请求
export interface HeuristicSearchRequest {
  // 待搜索的源字符串
  source: string;
  // 最小相似度阈值（0.0 ~ 1.0）
  min_similarity?: number;
  // 最大返回结果数
  max_results?: number;
}

// 启发式匹配结果
export interface HeuristicMatchDTO {
  // 候选源字符串
  source: string;
  // 候选翻译
  translation: string;
  // 归一化相似度 0.0~1.0
  similarity: number;
  // 编辑距离
  levenshtein: number;
  // 最长公共子串长度
  lcs_len: number;
}

// 翻译请求
export interface TranslateRequest {
  // 待翻译文本
  text: string;
  // 源语言（默认 "english"）
  source_lang?: string;
  // 目标语言（默认 "chinese"）
  target_lang?: string;
  // 翻译提供方（"openai" 或 "deepl"）
  provider?: string;
}

// 翻译提供方信息
export interface TranslationProvidersResponse {
  // 当前选中的提供方
  current: string;
  // 可用的提供方列表
  available: string[];
  // OpenAI 是否已配置
  openaiConfigured: boolean;
  // DeepL 是否已配置
  deeplConfigured: boolean;
  // 百度翻译是否已配置
  baiduConfigured: boolean;
  // 有道翻译是否已配置
  youdaoConfigured: boolean;
  // Azure 是否已配置
  azureConfigured: boolean;
  // Google 是否已配置
  googleConfigured: boolean;
}

// 虚拟滚动分页查询
///
// 用于前端虚拟滚动的分页加载。
// 返回当前视口的数据片段及统计信息。
export async function queryStrings(request: QueryRequest): Promise<QueryResponse> {
  return invoke("query_strings_command", { request });
}

// 获取统计信息
export async function getStats(): Promise<string> {
  return invoke("get_stats");
}

// 加载 ESP/ESM 文件
///
// 这是应用的核心命令，负责：
// 1. 解析 ESP/ESM 二进制文件
// 2. 加载关联的 Strings 文件
// 3. 构建 ESP 记录树（用于后续的回写操作）
// 4. 缓存解析结果以加速重复加载
///
// 参数：
// - `espPath`: ESP/ESM 文件的完整路径
// - `stringsDir`: Strings 文件所在目录（可选，默认使用 ESP 所在目录）
// - `language`: 字符串文件的语言标识（可选，默认 "english"）
// - `game`: 游戏类型（可选，用于加载正确的 record_defs）
///
// 返回：
// - `LoadEspResponse`: 包含解析统计和缓存状态
export async function loadEsp(
  espPath: string,
  stringsDir?: string,
  language?: string,
  game?: string,
): Promise<LoadEspResponse> {
  return invoke("load_esp", { espPath, stringsDir, language, game });
}

// 加载 SST 字典
///
// 使用 T1-T4 分层匹配算法将 SST 中的翻译应用到当前加载的字符串。
// 返回匹配统计信息。
export async function loadSst(sstPath: string): Promise<LoadSstResponse> {
  return invoke("load_sst", { sstPath });
}

// 保存 SST 字典
///
// 将当前加载的字符串保存为 SST 文件。
// 可选指定 master 文件列表（用于版本验证）。
export async function saveSst(
  sstPath: string,
  masters?: string[],
): Promise<void> {
  return invoke("save_sst", { sstPath, masters });
}

// 更新单个字符串的翻译
///
// 参数：
// - `id`: 字符串的内部稳定 ID
// - `translation`: 新的翻译文本
///
// 注意：此操作是本地的，不会立即保存到文件。
// 需要调用 saveSst() 或 saveStrings() 来持久化。
export async function updateTranslation(
  id: number,
  translation: string,
): Promise<void> {
  return invoke("update_translation", { id, translation });
}

// 批量更新字符串翻译
///
// 参数：
// - `updates`: 更新列表，每项为 [id, translation]
///
// 返回：
// - 成功更新的条目数
export async function batchUpdateTranslations(
  updates: [number, string][],
): Promise<number> {
  return invoke("batch_update_translations", { updates });
}

// 启发式搜索
///
// 在词汇库中搜索与给定源字符串相似的条目。
// 用于翻译建议和相似度计算。
export async function heuristicSearch(
  request: HeuristicSearchRequest,
): Promise<HeuristicMatchDTO[]> {
  return invoke("heuristic_search", { request });
}

// 翻译单个字符串
///
// 使用配置的翻译提供方（OpenAI / DeepL / 百度 / 有道 / Azure）
// 翻译给定的文本。
export async function translateString(
  request: TranslateRequest,
): Promise<string> {
  return invoke("translate_string", { request });
}

// 设置 OpenAI API Key
export async function setOpenAiApiKey(apiKey: string): Promise<void> {
  await invoke("set_openai_api_key", { apiKey });
  saveConfig({ openai_api_key: apiKey || undefined }).catch(() => {});
}

// 设置 DeepL API Key
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

// ── RTL Preview ───────────────────────────────────────────────────

export async function rtlPreview(text: string, applyReverse: boolean, applyShape: boolean, lineWidth: number): Promise<string[]> {
  return invoke("rtl_preview", { text, applyReverse, applyShape, lineWidth });
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
  exclude_keywords: HeaderKeywordDto[];
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

export async function headerRulesDelete(index: number): Promise<HeaderRuleDto[]> {
  return invoke("header_rules_delete", { index });
}

export async function headerRulesMove(index: number, direction: "up" | "down"): Promise<HeaderRuleDto[]> {
  return invoke("header_rules_move", { index, direction });
}

export async function headerRulesUpdate(index: number, field: string, value: string): Promise<HeaderRuleDto[]> {
  return invoke("header_rules_update", { index, field, value });
}

export async function headerRulesAdd(): Promise<HeaderRuleDto[]> {
  return invoke("header_rules_add");
}

// ── Template Manager ──────────────────────────────────────────────

export interface TemplateInfo {
  name: string;
  rule_count: number;
  enabled_count: number;
}

export async function headerTemplatesList(dir: string): Promise<TemplateInfo[]> {
  return invoke("header_templates_list", { dir });
}

export async function headerTemplatesSave(dir: string, name: string): Promise<void> {
  return invoke("header_templates_save", { dir, name });
}

export async function headerTemplatesLoad(dir: string, name: string): Promise<HeaderRuleDto[]> {
  return invoke("header_templates_load", { dir, name });
}

export async function headerTemplatesDelete(dir: string, name: string): Promise<void> {
  return invoke("header_templates_delete", { dir, name });
}

// ── Pre-Processing Options ─────────────────────────────────────────

export interface PreProcOptsDto {
  options: [string, string][];
}

export async function preprocOptsLoad(path: string): Promise<PreProcOptsDto> {
  return invoke("preproc_opts_load", { path });
}

export async function preprocOptsList(): Promise<PreProcOptsDto> {
  return invoke("preproc_opts_list");
}

export async function preprocOptsSet(key: string, value: string): Promise<PreProcOptsDto> {
  return invoke("preproc_opts_set", { key, value });
}

export async function preprocOptsDelete(key: string): Promise<PreProcOptsDto> {
  return invoke("preproc_opts_delete", { key });
}

export async function preprocOptsSave(path: string): Promise<void> {
  return invoke("preproc_opts_save", { path });
}

// ── Header Batch Wizard ───────────────────────────────────────────

export interface HeaderBatchConfig {
  source_dir: string;
  game_id: string;
  data_dir: string;
  create_backup: boolean;
}

export interface HeaderBatchProgress {
  current: number;
  total: number;
  file_path: string;
  strings_matched: number;
  stage: string;
  detail_count?: number | null;
  message: string;
}

export interface HeaderBatchComplete {
  total_files: number;
  success: number;
  failed: number;
  total_strings_matched: number;
  duration_ms: number;
  is_cancelled: boolean;
  errors: string[];
}

export async function headerBatchProcess(config: HeaderBatchConfig): Promise<HeaderBatchComplete> {
  return invoke("header_batch_process", { config });
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
