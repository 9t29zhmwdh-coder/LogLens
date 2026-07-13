pub mod custom_parser;
pub mod format_detector;
pub mod json_parser;
pub mod plaintext_parser;
pub mod kv_parser;
pub mod stacktrace_detector;

pub use custom_parser::CustomParserSet;
pub use format_detector::detect_format;
use crate::models::log_entry::{NormalizedEntry, LogSource, LogFormat};

/// If `source.parser_hint` names a configured custom parser and it matches
/// the line, use it. Otherwise (no hint, unknown hint, or the line simply
/// doesn't match that template's regex) fall through to auto-detection, so
/// a custom parser can never make a source silently drop lines it doesn't
/// recognize.
pub fn normalize_line(line: &str, source: &LogSource, custom_parsers: &CustomParserSet) -> Option<NormalizedEntry> {
    if let Some(hint) = &source.parser_hint {
        if let Some(parser) = custom_parsers.get(hint) {
            if let Some(entry) = custom_parser::parse(parser, line, source) {
                return Some(entry);
            }
        }
    }

    let format = detect_format(line);
    match format {
        LogFormat::Json => json_parser::parse(line, source),
        LogFormat::KeyValue => kv_parser::parse(line, source),
        _ => plaintext_parser::parse(line, source),
    }
}
