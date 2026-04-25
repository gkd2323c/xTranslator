use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use xt_core::strings::{StringsFile, StringsFormat};

pub fn load_strings(path: &str) -> Result<()> {
    let sf = StringsFile::load(path).with_context(|| format!("Failed to load: {}", path))?;

    let format_name = match sf.format {
        StringsFormat::NullTerminated => "Null-terminated (.STRINGS)",
        StringsFormat::LengthPrefixed => "Length-prefixed (.DLSTRINGS/.ILSTRINGS)",
    };

    println!("File: {}", path);
    println!("Format: {}", format_name);
    println!("Entries: {}", sf.len());
    println!();

    let mut entries: Vec<(&u32, &String)> = sf.strings.iter().collect();
    entries.sort_by_key(|(id, _)| **id);

    for (id, text) in entries.iter().take(20) {
        println!("  [{}] {}", id, text);
    }

    if entries.len() > 20 {
        println!("  ... ({} more entries)", entries.len() - 20);
    }

    Ok(())
}

pub fn save_strings(source: &str, dest: &str) -> Result<()> {
    let sf = StringsFile::load(source).with_context(|| format!("Failed to load: {}", source))?;

    let format = StringsFile::detect_format(Path::new(dest));
    sf.save_with_format(dest, format)
        .with_context(|| format!("Failed to save: {}", dest))?;

    println!("Saved {} entries to {}", sf.len(), dest);

    let saved_bytes = fs::metadata(dest)?.len();
    println!("File size: {} bytes", saved_bytes);

    Ok(())
}

pub fn modify_strings(path: &str, id: u32, text: &str) -> Result<()> {
    let mut sf = StringsFile::load(path).with_context(|| format!("Failed to load: {}", path))?;

    let old = sf.strings.get(&id).cloned();
    sf.strings.insert(id, text.to_string());

    sf.save(path).with_context(|| format!("Failed to save: {}", path))?;

    match old {
        Some(old_text) => println!("[{}] '{}' -> '{}'", id, old_text, text),
        None => println!("[{}] (new) -> '{}'", id, text),
    }

    Ok(())
}
