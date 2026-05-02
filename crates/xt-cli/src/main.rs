use anyhow::{Context, Result};
use std::env;

mod commands;

fn print_usage() {
    println!("xTranslator CLI - Phase 1");
    println!();
    println!("Usage: xt-cli <command> [args...]");
    println!();
    println!("Commands:");
    println!("  parse <input.esp/esm> [output.txt]        Parse ESP/ESM and extract strings");
    println!("  sst generate <output.sst>                Generate a test SST file");
    println!("  sst read <input.sst>                     Read and display SST contents");
    println!("  sst export <input.sst> <output.txt>      Export SST to tab-separated text");
    println!("  sst save <input.esp/esm> <output.sst>    Parse ESP and save as SST dictionary");
    println!("  sst roundtrip <input.sst> <output.sst>   Read+write+verify SST roundtrip");
    println!("  apply <input.esp/esm> <sst> [output.txt] Parse ESP and apply SST translation");
    println!("  apply-xml <input.esp/esm> <sst> <output.xml> Parse ESP, apply SST, export XML");
    println!(
        "  diff <esp/esm> <xml>                     Compare ESP parsing with Delphi XML export"
    );
    println!("  diff-xml <xml1> <xml2>                   Compare two XML exports");
    println!("  golden-diff <delphi-dir> <esp>            Cross-validate Rust vs Delphi output");
    println!("  strings load <file>                      Load and display strings file");
    println!(
        "  strings save <source> <dest>             Save strings to file (auto-detect format)"
    );
    println!("  strings modify <file> <id> <text>        Modify a string entry in-place");
    println!();
    println!("Examples:");
    println!("  xt-cli parse Skyrim.esm strings.txt");
    println!("  xt-cli apply Skyrim.esm translation.sst result.txt");
    println!("  xt-cli sst generate test.sst");
    println!("  xt-cli sst read test.sst");
    println!("  xt-cli sst save Skyrim.esm skyrim.sst");
    println!("  xt-cli sst roundtrip test.sst roundtrip.sst");
    println!("  xt-cli diff Skyrim.esm delphi_export.xml");
    println!("  xt-cli diff-xml rust_export.xml delphi_export.xml");
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "parse" => {
            if args.len() < 3 {
                println!("Usage: xt-cli parse <input.esp/esm> [output.txt]");
                return Ok(());
            }
            let output = args.get(3).map(|s| s.as_str());
            commands::parse::parse_esp(&args[2], output)?
        }
        "apply" => {
            if args.len() < 4 {
                println!("Usage: xt-cli apply <input.esp/esm> <sst> [output.txt]");
                return Ok(());
            }
            let output = args.get(4).map(|s| s.as_str());
            commands::parse::apply_sst(&args[2], &args[3], output)?
        }
        "apply-xml" => {
            if args.len() < 5 {
                println!("Usage: xt-cli apply-xml <input.esp/esm> <sst> <output.xml>");
                return Ok(());
            }
            commands::parse::apply_and_export_xml(&args[2], &args[3], &args[4])?
        }
        "diff" => {
            if args.len() < 4 {
                println!("Usage: xt-cli diff <esp/esm> <xml>");
                return Ok(());
            }
            commands::diff::diff_esp_with_xml(&args[2], &args[3])?
        }
        "diff-xml" => {
            if args.len() < 4 {
                println!("Usage: xt-cli diff-xml <xml1> <xml2>");
                return Ok(());
            }
            commands::diff::diff_xml_with_xml(&args[2], &args[3])?
        }
        "golden-diff" => {
            if args.len() < 4 {
                println!("Usage: xt-cli golden-diff <delphi-golden-dir> <esp>");
                println!("Example: xt-cli golden-diff tests/fixtures/delphi_golden Skyrim.esm");
                return Ok(());
            }
            commands::golden_diff::run_golden_diff(&args[2], &args[3])?
        }
        "strings" => {
            if args.len() < 4 {
                println!("Usage: xt-cli strings <load|save|modify> ...");
                return Ok(());
            }
            match args[2].as_str() {
                "load" => commands::strings_cmd::load_strings(&args[3])?,
                "save" => {
                    if args.len() < 5 {
                        println!("Usage: xt-cli strings save <source> <dest>");
                        return Ok(());
                    }
                    commands::strings_cmd::save_strings(&args[3], &args[4])?
                }
                "modify" => {
                    if args.len() < 6 {
                        println!("Usage: xt-cli strings modify <file> <id> <text>");
                        return Ok(());
                    }
                    let id: u32 = args[3].parse().context("Invalid string ID")?;
                    commands::strings_cmd::modify_strings(&args[4], id, &args[5])?
                }
                _ => println!("Unknown strings subcommand: {}", args[2]),
            }
        }
        "sst" => {
            if args.len() < 4 {
                println!("Usage: xt-cli sst <generate|read|export|save|roundtrip> <file> [output]");
                return Ok(());
            }
            match args[2].as_str() {
                "generate" => commands::sst::generate_test_sst(&args[3])?,
                "read" => commands::sst::read_sst(&args[3])?,
                "export" => {
                    if args.len() < 5 {
                        println!("Usage: xt-cli sst export <input.sst> <output.txt>");
                        return Ok(());
                    }
                    commands::sst::export_sst(&args[3], &args[4])?
                }
                "save" => {
                    if args.len() < 5 {
                        println!("Usage: xt-cli sst save <input.esp/esm> <output.sst>");
                        return Ok(());
                    }
                    commands::sst::save_sst(&args[3], &args[4], None)?
                }
                "roundtrip" => {
                    if args.len() < 5 {
                        println!("Usage: xt-cli sst roundtrip <input.sst> <output.sst>");
                        return Ok(());
                    }
                    commands::sst::roundtrip_sst(&args[3], &args[4])?
                }
                _ => println!("Unknown sst subcommand: {}", args[2]),
            }
        }
        _ => {
            println!("Unknown command: {}", args[1]);
            print_usage();
        }
    }

    Ok(())
}
