use std::time::Instant;
use xt_core::testing::generator::generate_test_data;
use xt_core::testing::query::query_strings;
use xt_shared::dto::{QueryRequest, QueryResponse, SkyStringDTO};

/// 模拟完整的 Tauri IPC 流程：
/// 1. 生成 10 万条数据
/// 2. 查询筛选
/// 3. 序列化为 JSON（模拟 IPC 传输）
/// 4. 反序列化
#[test]
fn benchmark_full_ipc_pipeline() {
    let data = generate_test_data(100_000);

    let request = QueryRequest {
        file_id: "test".to_string(),
        offset: 0,
        limit: 100,
        filter: Some("Dragon".to_string()),
        sort_field: Some("id".to_string()),
        sort_dir: Some("asc".to_string()),
        status_filter: None,
    };

    // 阶段 1: 后端查询
    let t1 = Instant::now();
    let result = query_strings(
        &data,
        request.filter.as_deref(),
        request.sort_field.as_deref(),
        request.sort_dir.as_deref(),
        request.offset,
        request.limit,
    );
    let query_ms = t1.elapsed().as_millis() as u64;

    // 阶段 2: 转换为 DTO
    let t2 = Instant::now();
    let dtos: Vec<SkyStringDTO> = result
        .items
        .iter()
        .map(|sk| SkyStringDTO {
            id: sk.id,
            source: sk.source.clone(),
            translation: sk.translation.clone(),
            record_sig: String::from_utf8_lossy(&sk.esp_ptr.record_sig).to_string(),
            field_sig: String::from_utf8_lossy(&sk.esp_ptr.field_sig).to_string(),
            form_id: format!("0x{:08X}", sk.esp_ptr.form_id),
            status: "translated".to_string(),
            list_index: 0,
            str_id: sk.esp_ptr.str_id,
            is_vmad: false,
            ld: 0,
        })
        .collect();
    let dto_ms = t2.elapsed().as_micros() as u64;

    // 阶段 3: 序列化为 JSON（模拟 IPC）
    let t3 = Instant::now();
    let response = QueryResponse {
        total: result.total,
        filtered: result.filtered,
        items: dtos,
        offset: request.offset,
        elapsed_ms: query_ms,
    };
    let json = serde_json::to_string(&response).unwrap();
    let json_ms = t3.elapsed().as_micros() as u64;

    // 阶段 4: 反序列化（模拟前端接收）
    let t4 = Instant::now();
    let _parsed: QueryResponse = serde_json::from_str(&json).unwrap();
    let parse_ms = t4.elapsed().as_micros() as u64;

    println!("\n========== IPC Benchmark ==========");
    println!("Data size: 100,000 items");
    println!("Filter: 'Dragon', Page: 100 items");
    println!("-----------------------------------");
    println!("Backend query:     {} ms", query_ms);
    println!("DTO conversion:    {} μs", dto_ms);
    println!("JSON serialize:    {} μs", json_ms);
    println!("JSON deserialize:  {} μs", parse_ms);
    println!("JSON payload size: {} bytes", json.len());
    println!("-----------------------------------");
    println!(
        "Total simulated:   {} ms",
        query_ms + (dto_ms + json_ms + parse_ms) / 1000
    );
    println!("===================================\n");

    // 验收标准：后端查询 < 100ms（当前目标），JSON 序列化 < 5ms
    assert!(query_ms < 500, "Query too slow: {}ms", query_ms); // 阶段 0 宽松标准
    assert!(json_ms < 5000, "JSON serialize too slow: {}μs", json_ms);
}

#[test]
fn benchmark_pagination_scenarios() {
    let data = generate_test_data(100_000);

    let scenarios = vec![
        ("No filter, page 1", None::<&str>, 0),
        ("No filter, page 500", None::<&str>, 50_000),
        ("Filter 'Iron', page 1", Some("Iron"), 0),
        ("Filter 'Dragon', page 1", Some("Dragon"), 0),
        ("Filter 'xyz', page 1", Some("xyz"), 0), // 无结果
    ];

    println!("\n========== Pagination Benchmark ==========");
    for (name, filter, offset) in scenarios {
        let t = Instant::now();
        let result = query_strings(&data, filter, None, None, offset, 100);
        let ms = t.elapsed().as_micros() as f64 / 1000.0;
        println!(
            "{:30} | filtered: {:6} | time: {:6.2} ms",
            name, result.filtered, ms
        );
    }
    println!("==========================================\n");
}
