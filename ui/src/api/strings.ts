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
}

export interface LoadSstResponse {
  matched: number;
  unmatched: number;
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

export async function setApiKey(apiKey: string): Promise<void> {
  return invoke("set_api_key", { apiKey });
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
