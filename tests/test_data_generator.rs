//! Test data generator for xTranslator
//!
//! Generates synthetic test data for reproducible testing
//! without requiring actual Skyrim installation.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use xt_core::types::esp_pointer::EspPointer;
use xt_core::types::params::SkyStringParams;
use xt_core::types::sky_string::SkyString;

fn make_sig(s: &str) -> [u8; 4] {
    let bytes = s.as_bytes();
    let mut sig = [0u8; 4];
    let len = bytes.len().min(4);
    sig[..len].copy_from_slice(&bytes[..len]);
    sig
}

pub struct TestDataGenerator {
    temp_dir: tempfile::TempDir,
}

impl TestDataGenerator {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            temp_dir: tempfile::TempDir::new()?,
        })
    }

    /// Generate synthetic SkyString data for testing
    pub fn generate_sky_strings(&self, count: usize) -> Vec<SkyString> {
        let record_sigs = [
            "INFO", "NPC_", "QUST", "BOOK", "WEAP", "ARMO", "MGEF", "ACTI",
        ];
        let mut strings = Vec::with_capacity(count);

        for i in 0..count {
            let source = if i % 10 == 0 {
                format!("Dragon shout {}", i + 1)
            } else if i % 5 == 0 {
                format!("Quest item {}", i + 1)
            } else {
                format!("Test string {}", i + 1)
            };

            let translation = if i % 3 == 0 {
                format!("龙吼 {}", i + 1)
            } else if i % 4 == 0 {
                format!("任务物品 {}", i + 1)
            } else {
                String::new()
            };

            let record_sig = make_sig(record_sigs[i % 8]);
            let field_sig = make_sig("FULL");

            let mut sk = SkyString::new((i + 1) as u32, source, translation, record_sig, field_sig);

            sk.list_index = (i % 3) as u8;
            sk.colab_id = (i % 256) as u8;

            if i % 3 == 0 {
                sk.params.set(SkyStringParams::TRANSLATED, true);
            }

            sk.esp_ptr = EspPointer {
                str_id: (i + 1) as i32,
                form_id: (1000 + i) as u32,
                record_sig,
                field_sig,
                index: 0,
                index_max: 0,
                edid_hash: 0,
            };

            strings.push(sk);
        }

        strings
    }

    /// Generate synthetic SST dictionary
    pub fn generate_sst_dictionary(&self, strings: &[SkyString]) -> anyhow::Result<String> {
        use xt_core::sst::v8::SstDictionary;

        let sst = SstDictionary::from_entries(strings.to_vec());
        let sst_path = self.temp_dir.path().join("test_dictionary.sst");
        sst.save_to_file(sst_path.to_str().unwrap())?;

        Ok(sst_path.to_string_lossy().to_string())
    }

    /// Generate synthetic XML export data
    pub fn generate_xml_export(&self, strings: &[SkyString]) -> anyhow::Result<String> {
        use std::io::Write;

        let xml_content = self.create_xml_content(strings)?;
        let xml_path = self.temp_dir.path().join("test_export.xml");
        let mut file = std::fs::File::create(&xml_path)?;
        file.write_all(xml_content.as_bytes())?;
        Ok(xml_path.to_string_lossy().to_string())
    }

    /// Generate synthetic Strings file
    pub fn generate_strings_file(&self, strings: &[SkyString]) -> anyhow::Result<String> {
        use xt_core::strings::StringsFile;
        use xt_core::strings::StringsFormat;

        let mut sf = StringsFile::new();
        sf.format = StringsFormat::NullTerminated;

        for s in strings {
            if !s.translation.is_empty() {
                sf.strings
                    .insert(s.esp_ptr.str_id as u32, s.translation.clone());
            }
        }

        let strings_path = self.temp_dir.path().join("test.strings");
        sf.save(&strings_path)?;
        Ok(strings_path.to_string_lossy().to_string())
    }

    /// Generate vocabulary file for testing
    pub fn generate_vocabulary_file(&self) -> anyhow::Result<String> {
        let vocab_content = r#"
# Test vocabulary file
Dragon=龙
Dragonborn=龙裔
Shout=龙吼
Thu'um=龙吼
Dovahkiin=龙裔
Civil War=内战
Imperial Legion=帝国军团
Stormcloaks=风暴斗篷
Skyrim=天际
Whiterun=白漫城
Solitude=独孤城
Windhelm=风舵城
Markarth=马卡斯城
Riften=裂谷城
Winterhold=冬堡
Dawnstar=晨星城
Morthal=莫萨尔城
Falkreath=佛克瑞斯城
"#;

        let vocab_path = self.temp_dir.path().join("vocabulary.txt");
        let mut file = std::fs::File::create(&vocab_path)?;
        file.write_all(vocab_content.as_bytes())?;
        Ok(vocab_path.to_string_lossy().to_string())
    }

    /// Generate test data configuration
    pub fn generate_test_config(&self) -> HashMap<String, String> {
        let mut config = HashMap::new();
        config.insert(
            "test_data_dir".to_string(),
            self.temp_dir.path().to_string_lossy().to_string(),
        );
        config.insert("string_count".to_string(), "1000".to_string());
        config.insert("translated_ratio".to_string(), "0.33".to_string());
        config.insert(
            "record_types".to_string(),
            "INFO,NPC_,QUST,BOOK,WEAP,ARMO,MGEF,ACTI".to_string(),
        );
        config
    }

    pub fn temp_dir_path(&self) -> &Path {
        self.temp_dir.path()
    }

    fn create_xml_content(&self, strings: &[SkyString]) -> anyhow::Result<String> {
        let mut xml = String::new();
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push_str("\n<strings>\n");

        for s in strings {
            if !s.translation.is_empty() {
                let rec = String::from_utf8_lossy(&s.record_sig);
                let fld = String::from_utf8_lossy(&s.field_sig);
                xml.push_str("  <string>\n");
                xml.push_str(&format!("    <id>{}</id>\n", s.esp_ptr.str_id));
                xml.push_str(&format!("    <record_sig>{}</record_sig>\n", rec));
                xml.push_str(&format!("    <field_sig>{}</field_sig>\n", fld));
                xml.push_str(&format!(
                    "    <source>{}</source>\n",
                    self.escape_xml(&s.source)
                ));
                xml.push_str(&format!(
                    "    <translation>{}</translation>\n",
                    self.escape_xml(&s.translation)
                ));
                xml.push_str("  </string>\n");
            }
        }

        xml.push_str("</strings>\n");
        Ok(xml)
    }

    fn escape_xml(&self, text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_generator_creation() {
        let generator = TestDataGenerator::new().unwrap();
        assert!(generator.temp_dir_path().exists());
    }

    #[test]
    fn test_generate_sky_strings() {
        let generator = TestDataGenerator::new().unwrap();
        let strings = generator.generate_sky_strings(100);

        assert_eq!(strings.len(), 100);

        // Check first string
        let first = &strings[0];
        assert!(first.source.contains("Dragon shout"));
        assert!(first.translation.contains("龙吼"));
        assert_eq!(first.record_sig, make_sig("INFO"));

        // Check variety
        let record_types: std::collections::HashSet<_> =
            strings.iter().map(|s| s.record_sig).collect();
        assert!(record_types.len() > 5);
    }

    #[test]
    fn test_generate_xml_export() {
        let generator = TestDataGenerator::new().unwrap();
        let strings = generator.generate_sky_strings(10);
        let xml_path = generator.generate_xml_export(&strings).unwrap();

        assert!(Path::new(&xml_path).exists());
        let xml_content = std::fs::read_to_string(&xml_path).unwrap();
        assert!(xml_content.contains("<?xml version=\"1.0\""));
        assert!(xml_content.contains("<strings>"));
        assert!(xml_content.contains("</strings>"));
        assert!(xml_content.contains("Dragon shout"));
        assert!(xml_content.contains("龙吼"));
    }

    #[test]
    fn test_generate_vocabulary_file() {
        let generator = TestDataGenerator::new().unwrap();
        let vocab_path = generator.generate_vocabulary_file().unwrap();

        assert!(Path::new(&vocab_path).exists());
        let vocab_content = std::fs::read_to_string(&vocab_path).unwrap();
        assert!(vocab_content.contains("Dragon=龙"));
        assert!(vocab_content.contains("Skyrim=天际"));
    }

    #[test]
    fn test_xml_escaping() {
        let generator = TestDataGenerator::new().unwrap();
        let input = r#"Test & < > " '"#;
        let escaped = generator.escape_xml(input);
        assert_eq!(escaped, "Test &amp; &lt; &gt; &quot; &apos;");
    }
}
