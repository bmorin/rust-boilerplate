#[allow(unused)]
use anyhow::{anyhow, Result};
use config::CONFIG;
use tokio::time::Duration;
#[allow(unused)]
use tracing::{debug, error, info, warn};

mod config;
mod logging;

// uncomment for the single threaded runtime
//#[tokio::main(flavor = "current_thread")]
#[tokio::main]
async fn main() -> Result<()> {
    let _log_flush_guard = logging::init();

    if CONFIG.load().systemd {
        // notify the systemd watchdog we're healthy every second, otherwise it will kill and restart us
        #[cfg(target_family = "unix")]
        let _systemd_watchdog = tokio::spawn(async move {
            info!("Starting systemd watchdog notifications");
            let mut interval = time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;

                // Panic if we fail to notify the watchdog
                systemd::daemon::notify(false, [(systemd::daemon::STATE_WATCHDOG, "1")].iter()).unwrap();
            }
        });
    }
    
    config::watch();

    info!("Process Started");
    info!("CONFIG = {CONFIG:?}");

    // Do the work here
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        info!("CONFIG = {CONFIG:?}");
    }
}
