use clap::{Parser, ValueEnum};
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use once_cell::sync::Lazy;
use parse_display::Display;
use serde::{Deserialize, Serialize};
use tokio::time::Duration;
#[allow(unused)]
use tracing::{debug, error, info, warn};

static CONFIGURATION_ENVIRONMENT_VARIABLE_PREFIX: &str = "MY_APP_";
static CONFIGURATION_FILE_NAME: &str = "MyApp.toml";

#[derive(Clone, Debug, Deserialize, Display, PartialEq, Serialize, ValueEnum)]
pub enum SampleEnum {
    Larry,
    Curly,
    Moe,
}

#[derive(Debug, Deserialize, Parser, Serialize)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Disable the console logger
    #[arg(long, default_value_t = false)]
    pub no_console: bool,

    /// Use the systemd watchdog
    #[arg(long, default_value_t = false)]
    pub systemd: bool,

    /// Directory to log to
    #[arg(long, default_value = "log")]
    pub log_dir: String,

    /// Example for the enum magic
    #[arg(long, default_value = "moe")]
    pub favorite_stooge: SampleEnum,
}

pub static CONFIG: Lazy<arc_swap::ArcSwap<Config>> = Lazy::new(|| {
    // Priority: defaults < command line < environment variables < config file
    arc_swap::ArcSwap::from_pointee(
        Figment::new()
            .merge(Serialized::defaults(Config::parse()))
            .merge(Env::prefixed(&CONFIGURATION_ENVIRONMENT_VARIABLE_PREFIX))
            .merge(Toml::file(CONFIGURATION_FILE_NAME))
            .extract::<Config>()
            // Panic if we fail to load the configuration on the first pass
            .unwrap(),
    )
});

/// Watch the configuration file for changes and update the inner value of CONFIG
pub fn watch() {
    let _hotload_config = tokio::spawn(async move {
        use futures::{SinkExt, StreamExt};
        use notify::Watcher;

        let (mut sender, receiver) = futures::channel::mpsc::channel(32);

        // This receives a synchronous callback that sends the result to the channel, so we use futures::executor::block_on
        // so we can use the asyncronous sender
        let mut watcher = notify::RecommendedWatcher::new(
            move |res| {
                futures::executor::block_on(async {
                    if let Err(e) = sender.send(res).await {
                        error!("Failed to send configuration file notification to channel: {e:?}")
                    }
                })
            },
            notify::Config::default(),
        )
        // Panic if we fail to create the watcher
        .unwrap();

        // Filter multiple events for the same file within a second
        let mut debounced_receiver = debounced::Debounced::new(receiver, Duration::from_secs(1));

        watcher
            .watch(
                CONFIGURATION_FILE_NAME.as_ref(),
                notify::RecursiveMode::NonRecursive,
            )
            // Panic if we fail to watch the configuration file
            .unwrap();

        while let Some(res) = debounced_receiver.next().await {
            match res {
                Ok(_) => {
                    match Figment::new()
                        .merge(Serialized::defaults(Config::parse()))
                        .merge(Env::prefixed(&CONFIGURATION_ENVIRONMENT_VARIABLE_PREFIX))
                        .merge(Toml::file(CONFIGURATION_FILE_NAME))
                        .extract::<Config>()
                    {
                        Ok(new_config) => {
                            info!("CONFIG UPDATED = {new_config:?}");
                            CONFIG.store(std::sync::Arc::new(new_config));
                        }
                        Err(e) => error!("Failed to parse updated configurtion: {e:?}"),
                    }
                }
                Err(e) => error!("Configuration file watch error: {e:?}"),
            }
        }
    });
}
