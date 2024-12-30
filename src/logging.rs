use crate::config::CONFIG;

// Environment variable for log level filtering with a default of INFO
static ENVIRIONMENT_VARIABLE: &str = "MY_APP_LOG";

/// Initialize the logging system
/// returns a guard that will flush the log on drop
pub fn init() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::prelude::*;

    let mut log_flush_guard: Option<tracing_appender::non_blocking::WorkerGuard> = None;
    let mut tracing_layers = Vec::new();
    let config = CONFIG.load();

    if !config.no_console {
        let console_timer =
            tracing_subscriber::fmt::time::ChronoLocal::new("%H:%M:%S:%6f".to_string());

        let console_layer = tracing_subscriber::fmt::layer()
            .with_timer(console_timer)
            .with_target(false)
            .boxed();
        tracing_layers.push(console_layer);
    }

    if !config.log_dir.is_empty() {
        let pid = std::process::id();
        let current_time = chrono::Local::now().format("%Y.%m.%d-%H.%M.%S");
        let file_appender = tracing_appender::rolling::never(
            &config.log_dir,
            format!("MyApp-{current_time}-{pid}.log"),
        );
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        log_flush_guard = Some(guard); // Make sure we flush on shutdown

        let file_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_ansi(false)
            .with_writer(non_blocking)
            .boxed();
        tracing_layers.push(file_layer);
    }

    tracing_subscriber::registry()
        .with(tracing_layers)
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_env_var(ENVIRIONMENT_VARIABLE)
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    log_flush_guard
}
