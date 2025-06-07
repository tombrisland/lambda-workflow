use lambda_runtime::tracing::log;
use lambda_runtime::tracing::log::{Level, Metadata, Record};

pub struct StdoutLogger;

const MISSING_VALUE: &str = "unknown";

impl log::Log for StdoutLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let module_path: &str = record.module_path().unwrap_or(MISSING_VALUE);
            let file: &str = record.file().unwrap_or(MISSING_VALUE);

            println!(
                "[{}:{}] {} - {}",
                module_path,
                file,
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}
