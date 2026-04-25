use crate::types::sky_string::SkyString;
use std::time::Instant;

/// 查询结果
pub struct QueryResult<'a> {
    pub total: u32,
    pub filtered: u32,
    pub items: Vec<&'a SkyString>,
    pub elapsed_ms: u64,
}

/// 在内存数据上做筛选、排序、分页
///
/// 性能目标：10 万条数据筛选 < 100ms
pub fn query_strings<'a>(
    data: &'a [SkyString],
    filter: Option<&str>,
    sort_field: Option<&str>,
    sort_dir: Option<&str>,
    offset: u32,
    limit: u32,
) -> QueryResult<'a> {
    let start = Instant::now();
    let total = data.len() as u32;

    // 1. 筛选（如果在 source/translation 中包含 filter 词）
    let mut filtered_data: Vec<&SkyString> = if let Some(filter_text) = filter {
        let ft = filter_text.to_lowercase();
        data.iter()
            .filter(|sk| {
                sk.source.to_lowercase().contains(&ft)
                    || sk.translation.to_lowercase().contains(&ft)
                    || std::str::from_utf8(&sk.esp_ptr.record_sig)
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&ft)
            })
            .collect()
    } else {
        data.iter().collect()
    };

    let filtered = filtered_data.len() as u32;

    // 2. 排序
    if let Some(field) = sort_field {
        let is_asc = sort_dir.as_deref() != Some("desc");
        match field {
            "id" => {
                if is_asc {
                    filtered_data.sort_by_key(|sk| sk.id);
                } else {
                    filtered_data.sort_by_key(|sk| std::cmp::Reverse(sk.id));
                }
            }
            "source" => {
                if is_asc {
                    filtered_data.sort_by(|a, b| a.source.cmp(&b.source));
                } else {
                    filtered_data.sort_by(|a, b| b.source.cmp(&a.source));
                }
            }
            "record_sig" => {
                if is_asc {
                    filtered_data.sort_by(|a, b| a.esp_ptr.record_sig.cmp(&b.esp_ptr.record_sig));
                } else {
                    filtered_data.sort_by(|a, b| b.esp_ptr.record_sig.cmp(&a.esp_ptr.record_sig));
                }
            }
            _ => {} // 默认不排序
        }
    }

    // 3. 分页
    let offset_usize = offset as usize;
    let limit_usize = limit as usize;
    let page: Vec<&SkyString> = filtered_data
        .into_iter()
        .skip(offset_usize)
        .take(limit_usize)
        .collect();

    let elapsed_ms = start.elapsed().as_millis() as u64;

    QueryResult {
        total,
        filtered,
        items: page,
        elapsed_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::generator::generate_test_data;

    #[test]
    fn test_query_no_filter() {
        let data = generate_test_data(1000);
        let result = query_strings(&data, None, None, None, 0, 50);
        assert_eq!(result.total, 1000);
        assert_eq!(result.filtered, 1000);
        assert_eq!(result.items.len(), 50);
    }

    #[test]
    fn test_query_with_filter() {
        let data = generate_test_data(10000);
        let result = query_strings(&data, Some("Iron"), None, None, 0, 100);
        assert_eq!(result.total, 10000);
        assert!(result.filtered < 10000);
        assert!(!result.items.is_empty());
    }

    #[test]
    fn test_query_performance_100k() {
        let data = generate_test_data(100_000);
        let result = query_strings(&data, Some("Dragon"), None, None, 0, 100);
        assert_eq!(result.total, 100_000);
        println!("Filter 'Dragon' on 100k items: {}ms", result.elapsed_ms);
        // 宽松标准：阶段 0 允许 < 500ms，后续优化到 < 100ms
        assert!(result.elapsed_ms < 500, "Filter too slow: {}ms", result.elapsed_ms);
    }
}
