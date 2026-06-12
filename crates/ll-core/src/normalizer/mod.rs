pub mod format_detector;
pub mod json_parser;
pub mod plaintext_parser;
pub mod kv_parser;
pub mod stacktrace_detector;

pub use format_detector::detect_format;
use crate::models::log_entry::{NormalizedEntry, LogSource, LogFormat};

pub fn normalize_line(line: &str, source: &LogSource) -> Option<NormalizedEntry> {
    let format = detect_format(line);
    match format {
        LogFormat::Json => json_parser::parse(line, source),
        LogFormat::KeyValue => kv_parser::parse(line, source),
        _ => plaintext_parser::parse(line, source),
    }
}
