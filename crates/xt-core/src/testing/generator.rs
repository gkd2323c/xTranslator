use crate::types::esp_pointer::string_hash;
use crate::types::params::SkyStringParams;
use crate::types::sky_string::SkyString;

/// 生成虚拟 SkyString 数据用于性能测试
pub fn generate_test_data(count: usize) -> Vec<SkyString> {
    let record_types: Vec<[u8; 4]> = vec![
        *b"INFO", *b"QUST", *b"BOOK", *b"ARMO", *b"WEAP",
        *b"MISC", *b"ALCH", *b"PERK", *b"DIAL", *b"NPC_",
    ];
    let field_types: Vec<[u8; 4]> = vec![
        *b"NAM1", *b"FULL", *b"DESC", *b"DNAM", *b"RNAM",
    ];
    let sample_words: Vec<&str> = vec![
        "Iron", "Sword", "Shield", "Potion", "Quest", "Dialog",
        "Armor", "Weapon", "Magic", "Dragon", "Town", "Guard",
        "Bandit", "Merchant", "Travel", "Adventure", "Legend",
        "Ancient", "Mysterious", "Powerful", "Hidden", "Secret",
        "Golden", "Silver", "Dark", "Light", "Fire", "Ice",
        "Storm", "Shadow", "Blood", "Steel", "Crystal", "Dragon",
    ];

    let mut items = Vec::with_capacity(count);

    for i in 0..count {
        let id = i as u32;
        let word_idx1 = i % sample_words.len();
        let word_idx2 = (i * 7 + 3) % sample_words.len();
        let word_idx3 = (i * 13 + 5) % sample_words.len();

        let source = format!(
            "{} {} {}",
            sample_words[word_idx1],
            sample_words[word_idx2],
            sample_words[word_idx3]
        );

        // 每第 3 个条目不翻译，模拟真实场景
        let translation = if i % 3 == 0 {
            String::new()
        } else {
            format!(
                "[{}] {} {} {}",
                i,
                sample_words[(word_idx1 + 1) % sample_words.len()],
                sample_words[(word_idx2 + 2) % sample_words.len()],
                sample_words[(word_idx3 + 3) % sample_words.len()]
            )
        };

        let rec_idx = i % record_types.len();
        let field_idx = i % field_types.len();

        let mut sk = SkyString::new(id, source, translation);
        sk.esp_ptr.record_sig = record_types[rec_idx];
        sk.esp_ptr.field_sig = field_types[field_idx];
        sk.esp_ptr.form_id = (0x01000000u32 + i as u32) & 0x00FFFFFF;
        sk.esp_ptr.str_id = i as i32;
        sk.esp_ptr.edid_hash = string_hash(&format!("Record_{}", i));

        // 设置状态
        if i % 3 == 0 {
            sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
        } else {
            sk.params.set(SkyStringParams::TRANSLATED, true);
        }

        items.push(sk);
    }

    items
}
