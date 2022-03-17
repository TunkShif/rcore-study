use log::{Level, LevelFilter, Log};

struct Logger;

static LOGGER: Logger = Logger;

pub fn init() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .unwrap()
}

impl Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let color = log_level_to_color(record.metadata().level());
            println!(
                "\x1b[{}m[{}] - {}\x1b[0m",
                color,
                record.metadata().level().as_str(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

fn log_level_to_color(level: log::Level) -> usize {
    match level {
        Level::Error => 31,
        Level::Debug => 32,
        Level::Info => 34,
        Level::Warn => 93,
        Level::Trace => 90,
    }
}
