//! Test data generator for xTranslator
//! 
//! Generates synthetic ESP/ESM files and test data for reproducible testing
//! without requiring actual Skyrim installation

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;
use xt_core::types::sky_string::SkyString;
use xt_core::types::esp_pointer::EspPointer;
use xt_core::types::params::StringParams;
use xt_core::types::search_index::SearchIndex;

pub struct TestDataGenerator {
    temp_dir: TempDir,
}

impl TestDataGenerator {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            temp_dir: TempDir::new()?,
        })
    }

    /// Generate synthetic SkyString data for testing
    pub fn generate_sky_strings(&self, count: usize) -> Vec<SkyString> {
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

            let record_sig = match i % 8 {
                0 => "INFO",
                1 => "NPC_",
                2 => "QUST",
                3 => "BOOK",
                4 => "WEAP",
                5 => "ARMO",
                6 => "MGEF",
                _ => "ACTI",
            };

            strings.push(SkyString {
                id: (i + 1) as u32,
                source: source.clone(),
                translation,
                record_sig: record_sig.to_string(),
                field_sig: "FULL".to_string(),
                form_id: format!("0x{:x}", 1000 + i),
                status: if i % 3 == 0 { "translated" } else { "untranslated" }.to_string(),
                list_index: i,
                str_id: (i + 1) as u32,
                is_vmad: i % 15 == 0,
                ld: 0,
                esp_ptr: EspPointer {
                    str_id: (i + 1) as u32,
                    record_sig: record_sig.to_string(),
                    field_sig: "FULL".to_string(),
                    compressed: i % 2 == 0,
                },
                params: StringParams::new(),
                search_index: SearchIndex::new(&source),
            });
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
        let xml_content = self.create_xml_content(strings)?;
        let xml_path = self.temp_dir.path().join("test_export.xml");
        
        let mut file = File::create(&xml_path)?;
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
                sf.strings.insert(s.esp_ptr.str_id as u32, s.translation.clone());
            }
        }
        
        let strings_path = self.temp_dir.path().join("test.strings");
        sf.save(&strings_path)?;
        
        Ok(strings_path.to_string_lossy().to_string())
    }

    /// Create minimal ESP file structure for testing
    pub fn generate_minimal_esp(&self, strings: &[SkyString]) -> anyhow::Result<String> {
        // This would require implementing ESP file generation
        // For now, create a placeholder file
        let esp_path = self.temp_dir.path().join("test.esp");
        let mut file = File::create(&esp_path)?;
        
        // Write minimal ESP header
        file.write_all(b"TES4")?;
        file.write_all(&[0x08, 0x00, 0x00, 0x00])?; // Header size
        file.write_all(&[0x28, 0x00, 0x00, 0x00])?; // Flags
        file.write_all(&[0x00, 0x00, 0x00, 0x00])?; // FormID
        file.write_all(&[0x00, 0x00, 0x00, 0x00])?; // Unknown
        
        // Write some placeholder data
        for _ in 0..100 {
            file.write_all(&[0x00, 0x00, 0x00, 0x00])?;
        }
        
        Ok(esp_path.to_string_lossy().to_string())
    }

    /// Generate vocabulary file for testing
    pub fn generate_vocabulary_file(&self) -> anyhow::Result<String> {
        let vocab_content = r#"
# Test vocabulary file
# Format: STRINGS=Translation

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
        let mut file = File::create(&vocab_path)?;
        file.write_all(vocab_content.as_bytes())?;
        
        Ok(vocab_path.to_string_lossy().to_string())
    }

    /// Generate test data configuration
    pub fn generate_test_config(&self) -> HashMap<String, String> {
        let mut config = HashMap::new();
        
        config.insert("test_data_dir".to_string(), self.temp_dir.path().to_string_lossy().to_string());
        config.insert("string_count".to_string(), "1000".to_string());
        config.insert("translated_ratio".to_string(), "0.33".to_string());
        config.insert("record_types".to_string(), "INFO,NPC_,QUST,BOOK,WEAP,ARMO,MGEF,ACTI".to_string());
        
        config
    }

    /// Get temporary directory path
    pub fn temp_dir_path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Create XML content from strings
    fn create_xml_content(&self, strings: &[SkyString]) -> anyhow::Result<String> {
        let mut xml = String::new();
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push_str("\n<strings>\n");
        
        for s in strings {
            if !s.translation.is_empty() {
                xml.push_str("  <string>\n");
                xml.push_str(&format!("    <id>{}</id>\n", s.str_id));
                xml.push_str(&format!("    <record_sig>{}</record_sig>\n", s.record_sig));
                xml.push_str(&format!("    <field_sig>{}</field_sig>\n", s.field_sig));
                xml.push_str(&format!("    <source>{}</source>\n", self.escape_xml(&s.source)));
                xml.push_str(&format!("    <translation>{}</translation>\n", self.escape_xml(&s.translation)));
                xml.push_str("  </string>\n");
            }
        }
        
        xml.push_str("</strings>\n");
        Ok(xml)
    }

    /// Escape XML special characters
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
        assert_eq!(first.record_sig, "INFO");
        assert_eq!(first.status, "translated");
        
        // Check variety
        let record_types: std::collections::HashSet<_> = strings.iter()
            .map(|s| &s.record_sig)
            .collect();
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
