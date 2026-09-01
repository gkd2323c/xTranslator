use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandProcessorScript {
    pub globals: CommandProcessorGlobals,
    pub rules: Vec<CommandRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandProcessorGlobals {
    pub vocab_folder: Option<String>,
    pub import_folder: Option<String>,
    pub export_folder: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRule {
    pub line: usize,
    pub lang_source: Option<String>,
    pub lang_dest: Option<String>,
    pub use_data_dir: bool,
    pub export_subfolder: Option<String>,
    pub commands: Vec<ProcessorCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessorCommand {
    pub line: usize,
    pub kind: ProcessorCommandKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessorCommandKind {
    LoadFile {
        path: String,
    },
    CloseFile,
    CloseAll,
    Finalize,
    GenerateDictionaries,
    ApplySst {
        compare_option: u8,
        apply_mode: u8,
        path: String,
    },
    ImportSst {
        compare_option: u8,
        apply_mode: u8,
        path: String,
    },
    ImportXml {
        compare_option: u8,
        apply_mode: u8,
        path: String,
    },
    LoadMasters,
    SaveDictionary,
    ApiTranslation {
        api_id: u8,
        auto_no_trans_tag: bool,
    },
}

impl ProcessorCommandKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::LoadFile { .. } => "LoadFile",
            Self::CloseFile => "CloseFile",
            Self::CloseAll => "CloseAll",
            Self::Finalize => "Finalize",
            Self::GenerateDictionaries => "GenerateDictionaries",
            Self::ApplySst { .. } => "ApplySst",
            Self::ImportSst { .. } => "ImportSst",
            Self::ImportXml { .. } => "ImportXml",
            Self::LoadMasters => "LoadMasters",
            Self::SaveDictionary => "SaveDictionary",
            Self::ApiTranslation { .. } => "ApiTranslation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandErrorPolicy {
    #[default]
    Stop,
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandExecutionFailure {
    pub rule_number: usize,
    pub command_number: Option<usize>,
    pub line: usize,
    pub command: Option<&'static str>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandExecutionReport {
    pub rules_started: usize,
    pub rules_completed: usize,
    pub commands_succeeded: usize,
    pub failures: Vec<CommandExecutionFailure>,
    pub stopped_early: bool,
}

#[async_trait]
pub trait CommandProcessorHost: Send {
    async fn begin_rule(
        &mut self,
        globals: &CommandProcessorGlobals,
        rule_number: usize,
        rule: &CommandRule,
    ) -> Result<(), String>;

    async fn execute_command(
        &mut self,
        globals: &CommandProcessorGlobals,
        rule_number: usize,
        command_number: usize,
        rule: &CommandRule,
        command: &ProcessorCommand,
    ) -> Result<(), String>;

    async fn end_rule(
        &mut self,
        _globals: &CommandProcessorGlobals,
        _rule_number: usize,
        _rule: &CommandRule,
    ) -> Result<(), String> {
        Ok(())
    }
}

pub async fn execute_command_processor<H: CommandProcessorHost>(
    script: &CommandProcessorScript,
    host: &mut H,
    error_policy: CommandErrorPolicy,
) -> CommandExecutionReport {
    let mut report = CommandExecutionReport::default();

    for (rule_index, rule) in script.rules.iter().enumerate() {
        let rule_number = rule_index + 1;
        report.rules_started += 1;

        if let Err(message) = host.begin_rule(&script.globals, rule_number, rule).await {
            report.failures.push(CommandExecutionFailure {
                rule_number,
                command_number: None,
                line: rule.line,
                command: None,
                message,
            });
            if error_policy == CommandErrorPolicy::Stop {
                report.stopped_early = true;
                break;
            }
            continue;
        }

        let mut rule_failed = false;
        for (command_index, command) in rule.commands.iter().enumerate() {
            let command_number = command_index + 1;
            match host
                .execute_command(&script.globals, rule_number, command_number, rule, command)
                .await
            {
                Ok(()) => report.commands_succeeded += 1,
                Err(message) => {
                    rule_failed = true;
                    report.failures.push(CommandExecutionFailure {
                        rule_number,
                        command_number: Some(command_number),
                        line: command.line,
                        command: Some(command.kind.name()),
                        message,
                    });
                    if error_policy == CommandErrorPolicy::Stop {
                        report.stopped_early = true;
                        return report;
                    }
                }
            }
        }

        if let Err(message) = host.end_rule(&script.globals, rule_number, rule).await {
            rule_failed = true;
            report.failures.push(CommandExecutionFailure {
                rule_number,
                command_number: None,
                line: rule.line,
                command: None,
                message,
            });
            if error_policy == CommandErrorPolicy::Stop {
                report.stopped_early = true;
                return report;
            }
        }

        if !rule_failed {
            report.rules_completed += 1;
        }
    }

    report
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CommandProcessorParseError {
    #[error("line {line}: EndRule without matching StartRule")]
    UnexpectedEndRule { line: usize },
    #[error("line {line}: nested StartRule is not allowed")]
    NestedStartRule { line: usize },
    #[error("line {line}: StartRule has no matching EndRule")]
    UnterminatedRule { line: usize },
    #[error("line {line}: Command= is only valid inside StartRule/EndRule")]
    CommandOutsideRule { line: usize },
    #[error("line {line}: unknown command '{command}'")]
    UnknownCommand { line: usize, command: String },
    #[error("line {line}: command '{command}' is missing {parameter}")]
    MissingParameter {
        line: usize,
        command: String,
        parameter: &'static str,
    },
    #[error("line {line}: command '{command}' has invalid {parameter}: '{value}'")]
    InvalidParameter {
        line: usize,
        command: String,
        parameter: &'static str,
        value: String,
    },
}

#[derive(Debug)]
struct RuleBuilder {
    line: usize,
    lang_source: Option<String>,
    lang_dest: Option<String>,
    use_data_dir: bool,
    export_subfolder: Option<String>,
    commands: Vec<ProcessorCommand>,
}

impl RuleBuilder {
    fn new(line: usize) -> Self {
        Self {
            line,
            lang_source: None,
            lang_dest: None,
            use_data_dir: true,
            export_subfolder: None,
            commands: Vec::new(),
        }
    }

    fn finish(self) -> CommandRule {
        CommandRule {
            line: self.line,
            lang_source: self.lang_source,
            lang_dest: self.lang_dest,
            use_data_dir: self.use_data_dir,
            export_subfolder: self.export_subfolder,
            commands: self.commands,
        }
    }
}

pub fn parse_command_processor(
    text: &str,
) -> Result<CommandProcessorScript, CommandProcessorParseError> {
    let mut script = CommandProcessorScript::default();
    let mut current_rule: Option<RuleBuilder> = None;

    for (line_index, raw_line) in text.lines().enumerate() {
        let line = line_index + 1;
        if raw_line.trim().is_empty() || raw_line.starts_with('#') {
            continue;
        }

        let trimmed = raw_line.trim();
        if trimmed.eq_ignore_ascii_case("StartRule") {
            if current_rule.is_some() {
                return Err(CommandProcessorParseError::NestedStartRule { line });
            }
            current_rule = Some(RuleBuilder::new(line));
            continue;
        }
        if trimmed.eq_ignore_ascii_case("EndRule") {
            let rule = current_rule
                .take()
                .ok_or(CommandProcessorParseError::UnexpectedEndRule { line })?;
            script.rules.push(rule.finish());
            continue;
        }

        if let Some(command_text) = strip_prefix_ascii_case(trimmed, "Command=") {
            let rule = current_rule
                .as_mut()
                .ok_or(CommandProcessorParseError::CommandOutsideRule { line })?;
            rule.commands.push(ProcessorCommand {
                line,
                kind: parse_command(command_text.trim(), line)?,
            });
            continue;
        }

        let Some((name, value)) = trimmed.split_once('=') else {
            // Delphi ignores decorative/unknown lines in processor files.
            continue;
        };
        let name = name.trim();
        let value = value.trim();

        if let Some(rule) = current_rule.as_mut() {
            if name.eq_ignore_ascii_case("langsource") {
                rule.lang_source = non_empty(value);
            } else if name.eq_ignore_ascii_case("langdest") {
                rule.lang_dest = non_empty(value);
            } else if name.eq_ignore_ascii_case("usedatadir") {
                rule.use_data_dir = parse_delphi_bool(value).unwrap_or(true);
            } else if name.eq_ignore_ascii_case("exportsubfolder") {
                rule.export_subfolder = non_empty(value);
            }
        } else if name.eq_ignore_ascii_case("global_vocabfolder") {
            script.globals.vocab_folder = non_empty(value);
        } else if name.eq_ignore_ascii_case("global_importfolder") {
            script.globals.import_folder = non_empty(value);
        } else if name.eq_ignore_ascii_case("global_exportfolder") {
            script.globals.export_folder = non_empty(value);
        }
    }

    if let Some(rule) = current_rule {
        return Err(CommandProcessorParseError::UnterminatedRule { line: rule.line });
    }

    Ok(script)
}

fn parse_command(
    command: &str,
    line: usize,
) -> Result<ProcessorCommandKind, CommandProcessorParseError> {
    let (name, args) = command
        .split_once(':')
        .map_or((command, None), |(name, rest)| (name, Some(rest)));
    let normalized = name.trim().to_ascii_lowercase();

    match normalized.as_str() {
        "loadfile" => Ok(ProcessorCommandKind::LoadFile {
            path: require_text(args, line, name, "path")?.to_string(),
        }),
        "closefile" => Ok(ProcessorCommandKind::CloseFile),
        "closeall" => Ok(ProcessorCommandKind::CloseAll),
        "finalize" => Ok(ProcessorCommandKind::Finalize),
        "generatedictionaries" => Ok(ProcessorCommandKind::GenerateDictionaries),
        "applysst" => parse_import_command(args, line, name, ImportKind::ApplySst),
        "importsst" => parse_import_command(args, line, name, ImportKind::ImportSst),
        "importxml" => parse_import_command(args, line, name, ImportKind::ImportXml),
        "loadmasters" => Ok(ProcessorCommandKind::LoadMasters),
        "savedictionary" => Ok(ProcessorCommandKind::SaveDictionary),
        "apitranslation" => parse_api_translation(args, line, name),
        _ => Err(CommandProcessorParseError::UnknownCommand {
            line,
            command: name.trim().to_string(),
        }),
    }
}

enum ImportKind {
    ApplySst,
    ImportSst,
    ImportXml,
}

fn parse_import_command(
    args: Option<&str>,
    line: usize,
    command: &str,
    kind: ImportKind,
) -> Result<ProcessorCommandKind, CommandProcessorParseError> {
    let args = require_text(args, line, command, "compare option, apply mode and path")?;
    let mut parts = args.splitn(3, ':');
    let compare_option = parse_u8(parts.next(), line, command, "compare option")?;
    let apply_mode = parse_u8(parts.next(), line, command, "apply mode")?;
    let path = require_text(parts.next(), line, command, "path")?.to_string();

    Ok(match kind {
        ImportKind::ApplySst => ProcessorCommandKind::ApplySst {
            compare_option,
            apply_mode,
            path,
        },
        ImportKind::ImportSst => ProcessorCommandKind::ImportSst {
            compare_option,
            apply_mode,
            path,
        },
        ImportKind::ImportXml => ProcessorCommandKind::ImportXml {
            compare_option,
            apply_mode,
            path,
        },
    })
}

fn parse_api_translation(
    args: Option<&str>,
    line: usize,
    command: &str,
) -> Result<ProcessorCommandKind, CommandProcessorParseError> {
    let args = require_text(args, line, command, "API id and auto-no-translation-tag flag")?;
    let mut parts = args.splitn(2, ':');
    let api_id = parse_u8(parts.next(), line, command, "API id")?;
    let no_trans_raw = require_text(parts.next(), line, command, "auto-no-translation-tag flag")?;
    let auto_no_trans_tag = parse_delphi_bool(no_trans_raw).ok_or_else(|| {
        CommandProcessorParseError::InvalidParameter {
            line,
            command: command.to_string(),
            parameter: "auto-no-translation-tag flag",
            value: no_trans_raw.to_string(),
        }
    })?;

    Ok(ProcessorCommandKind::ApiTranslation {
        api_id,
        auto_no_trans_tag,
    })
}

fn parse_u8(
    value: Option<&str>,
    line: usize,
    command: &str,
    parameter: &'static str,
) -> Result<u8, CommandProcessorParseError> {
    let value = require_text(value, line, command, parameter)?;
    value
        .trim()
        .parse::<u8>()
        .map_err(|_| CommandProcessorParseError::InvalidParameter {
            line,
            command: command.to_string(),
            parameter,
            value: value.to_string(),
        })
}

fn require_text<'a>(
    value: Option<&'a str>,
    line: usize,
    command: &str,
    parameter: &'static str,
) -> Result<&'a str, CommandProcessorParseError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CommandProcessorParseError::MissingParameter {
            line,
            command: command.to_string(),
            parameter,
        })
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn strip_prefix_ascii_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map(|_| &text[prefix.len()..])
}

fn parse_delphi_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeHost {
        fail_on: Option<&'static str>,
        seen: Vec<String>,
    }

    #[async_trait]
    impl CommandProcessorHost for FakeHost {
        async fn begin_rule(
            &mut self,
            _globals: &CommandProcessorGlobals,
            rule_number: usize,
            _rule: &CommandRule,
        ) -> Result<(), String> {
            self.seen.push(format!("begin:{rule_number}"));
            Ok(())
        }

        async fn execute_command(
            &mut self,
            _globals: &CommandProcessorGlobals,
            rule_number: usize,
            command_number: usize,
            _rule: &CommandRule,
            command: &ProcessorCommand,
        ) -> Result<(), String> {
            let name = command.kind.name();
            self.seen
                .push(format!("command:{rule_number}:{command_number}:{name}"));
            if self.fail_on == Some(name) {
                Err(format!("{name} failed"))
            } else {
                Ok(())
            }
        }

        async fn end_rule(
            &mut self,
            _globals: &CommandProcessorGlobals,
            rule_number: usize,
            _rule: &CommandRule,
        ) -> Result<(), String> {
            self.seen.push(format!("end:{rule_number}"));
            Ok(())
        }
    }

    #[test]
    fn parses_delphi_processor_sample_and_preserves_windows_paths() {
        let script = parse_command_processor(
            r#"
# processor sample
Global_VocabFolder=C:\Vocab
Global_ImportFolder=C:\Imports
Global_ExportFolder=C:\Exports

StartRule
LangSource=english
LangDest=chinese
UseDataDir=false
ExportSubFolder=Translated
Command=LoadFile:C:\Games\Skyrim Special Edition\Data\Example.esp
Command=ApplySst:0:1:Example.esp
Command=ImportSst:2:3:C:\Imports\Example.sst
Command=ImportXml:1:0:C:\Imports\Example.xml
Command=ApiTranslation:5:1
Command=Finalize
Command=CloseFile
EndRule
"#,
        )
        .expect("sample processor should parse");

        assert_eq!(script.globals.vocab_folder.as_deref(), Some(r"C:\Vocab"));
        assert_eq!(script.globals.import_folder.as_deref(), Some(r"C:\Imports"));
        assert_eq!(script.globals.export_folder.as_deref(), Some(r"C:\Exports"));
        assert_eq!(script.rules.len(), 1);

        let rule = &script.rules[0];
        assert_eq!(rule.lang_source.as_deref(), Some("english"));
        assert_eq!(rule.lang_dest.as_deref(), Some("chinese"));
        assert!(!rule.use_data_dir);
        assert_eq!(rule.export_subfolder.as_deref(), Some("Translated"));
        assert_eq!(rule.commands.len(), 7);
        assert_eq!(
            rule.commands[0].kind,
            ProcessorCommandKind::LoadFile {
                path: r"C:\Games\Skyrim Special Edition\Data\Example.esp".to_string()
            }
        );
        assert_eq!(
            rule.commands[1].kind,
            ProcessorCommandKind::ApplySst {
                compare_option: 0,
                apply_mode: 1,
                path: "Example.esp".to_string(),
            }
        );
        assert_eq!(
            rule.commands[4].kind,
            ProcessorCommandKind::ApiTranslation {
                api_id: 5,
                auto_no_trans_tag: true,
            }
        );
    }

    #[test]
    fn defaults_use_data_dir_to_true_like_delphi() {
        let script = parse_command_processor(
            "StartRule\nUseDataDir=not-a-bool\nCommand=CloseAll\nEndRule\n",
        )
        .expect("Delphi defaults invalid UseDataDir to true");

        assert!(script.rules[0].use_data_dir);
    }

    #[test]
    fn command_keywords_are_case_insensitive() {
        let script = parse_command_processor(
            "sTaRtRuLe\nCoMmAnD=LoAdMaStErS\ncommand=SaveDictionary\neNdRuLe\n",
        )
        .expect("processor keywords should be case insensitive");

        assert_eq!(
            script.rules[0]
                .commands
                .iter()
                .map(|command| &command.kind)
                .collect::<Vec<_>>(),
            vec![
                &ProcessorCommandKind::LoadMasters,
                &ProcessorCommandKind::SaveDictionary,
            ]
        );
    }

    #[test]
    fn reports_unknown_command_with_source_line() {
        let error = parse_command_processor(
            "StartRule\nCommand=LoadFile:test.esp\nCommand=RunShell:del *\nEndRule\n",
        )
        .expect_err("unknown commands must be rejected");

        assert_eq!(
            error,
            CommandProcessorParseError::UnknownCommand {
                line: 3,
                command: "RunShell".to_string(),
            }
        );
    }

    #[test]
    fn reports_structural_errors_with_source_line() {
        assert_eq!(
            parse_command_processor("EndRule\n").expect_err("orphan EndRule must fail"),
            CommandProcessorParseError::UnexpectedEndRule { line: 1 }
        );
        assert_eq!(
            parse_command_processor("StartRule\nStartRule\n")
                .expect_err("nested StartRule must fail"),
            CommandProcessorParseError::NestedStartRule { line: 2 }
        );
        assert_eq!(
            parse_command_processor("StartRule\nCommand=CloseAll\n")
                .expect_err("unterminated rule must fail"),
            CommandProcessorParseError::UnterminatedRule { line: 1 }
        );
    }

    #[test]
    fn command_outside_rule_is_rejected() {
        assert_eq!(
            parse_command_processor("Command=CloseAll\n")
                .expect_err("commands outside rules must fail"),
            CommandProcessorParseError::CommandOutsideRule { line: 1 }
        );
    }

    #[test]
    fn malformed_import_parameters_are_reported() {
        let error = parse_command_processor("StartRule\nCommand=ApplySst:x:1:test.sst\nEndRule\n")
            .expect_err("invalid numeric parameters must fail");

        assert_eq!(
            error,
            CommandProcessorParseError::InvalidParameter {
                line: 2,
                command: "ApplySst".to_string(),
                parameter: "compare option",
                value: "x".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn executor_stops_and_reports_exact_rule_command_and_line() {
        let script = parse_command_processor(
            "StartRule\nCommand=LoadFile:test.esp\nCommand=ImportXml:0:1:test.xml\nCommand=Finalize\nEndRule\n",
        )
        .expect("script should parse");
        let mut host = FakeHost {
            fail_on: Some("ImportXml"),
            seen: Vec::new(),
        };

        let report = execute_command_processor(&script, &mut host, CommandErrorPolicy::Stop).await;

        assert_eq!(report.commands_succeeded, 1);
        assert!(report.stopped_early);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(
            report.failures[0],
            CommandExecutionFailure {
                rule_number: 1,
                command_number: Some(2),
                line: 3,
                command: Some("ImportXml"),
                message: "ImportXml failed".to_string(),
            }
        );
        assert_eq!(
            host.seen,
            vec!["begin:1", "command:1:1:LoadFile", "command:1:2:ImportXml",]
        );
    }

    #[tokio::test]
    async fn executor_can_continue_after_failure() {
        let script = parse_command_processor(
            "StartRule\nCommand=LoadFile:test.esp\nCommand=Finalize\nEndRule\n",
        )
        .expect("script should parse");
        let mut host = FakeHost {
            fail_on: Some("LoadFile"),
            seen: Vec::new(),
        };

        let report =
            execute_command_processor(&script, &mut host, CommandErrorPolicy::Continue).await;

        assert_eq!(report.commands_succeeded, 1);
        assert_eq!(report.failures.len(), 1);
        assert!(!report.stopped_early);
        assert_eq!(report.rules_completed, 0);
        assert_eq!(host.seen.last().map(String::as_str), Some("end:1"));
    }
}
