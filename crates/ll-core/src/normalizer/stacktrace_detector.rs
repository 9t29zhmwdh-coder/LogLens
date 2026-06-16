use once_cell::sync::Lazy;
use regex::Regex;

static CONTINUATION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?x)
        ^\s{2,}                          |  # indented (Python/Java)
        ^\tat\s                           |  # Java: \tat com.example...
        ^at\s+\S+\s*\(                   |  # JS: at fn (file:line:col)
        ^\s*File\s+\"                     |  # Python: File \"path\"
        ^\s*Caused\s+by:                  |  # Java chained exception
        ^\s*\d+:\s+0x[0-9a-f]+\s+-\s+   |  # Rust backtrace:  0: 0x... - fn
        ^\s+\d+\)\s+0x[0-9a-f]+          |  # macOS backtrace
        ^\s+\.\.\.\s+\d+\s+more          |  # Java: ... N more
        ^note:                            |  # Rust note:
        ^caused by:                          # lowercase variant
    "#).unwrap()
});

pub fn is_continuation(line: &str) -> bool {
    CONTINUATION.is_match(line)
}

/// Accumulates lines into a complete log entry, handling multi-line stacktraces.
pub struct StacktraceAccumulator {
    pending: Option<String>,
    stacktrace: Vec<String>,
}

impl StacktraceAccumulator {
    pub fn new() -> Self {
        Self { pending: None, stacktrace: Vec::new() }
    }

    /// Push a line; returns a complete (message, stacktrace) pair if an entry is finalized.
    pub fn push(&mut self, line: &str) -> Option<(String, Vec<String>)> {
        if is_continuation(line) {
            self.stacktrace.push(line.to_string());
            None
        } else {
            let result = self.pending.take().map(|msg| (msg, std::mem::take(&mut self.stacktrace)));
            self.pending = Some(line.to_string());
            result
        }
    }

    pub fn flush(&mut self) -> Option<(String, Vec<String>)> {
        self.pending.take().map(|msg| (msg, std::mem::take(&mut self.stacktrace)))
    }
}

impl Default for StacktraceAccumulator {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_java_stacktrace() {
        assert!(is_continuation("\tat com.example.MyClass.method(MyClass.java:42)"));
        assert!(is_continuation("    at com.example.Foo.bar(Foo.java:10)"));
    }

    #[test]
    fn detects_rust_backtrace() {
        assert!(is_continuation("   0: 0x10a2b4c - std::panicking::begin_panic"));
    }

    #[test]
    fn detects_python_traceback() {
        assert!(is_continuation("  File \"app.py\", line 42, in handler"));
    }

    #[test]
    fn normal_line_not_continuation() {
        assert!(!is_continuation("2024-01-01 ERROR Something went wrong"));
        assert!(!is_continuation("{\"level\":\"error\",\"msg\":\"oops\"}"));
    }
}
