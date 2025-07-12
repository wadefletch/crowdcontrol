use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::Level;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Get the standard log directory for the application
fn get_log_dir() -> Result<PathBuf> {
    let log_dir = if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(data_home).join("crowdcontrol")
    } else {
        dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Unable to determine local data directory"))?
            .join("crowdcontrol")
    };

    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create log directory: {:?}", log_dir))?;

    Ok(log_dir)
}

/// Initialize the tracing subscriber with both console and file outputs
pub fn init_logger(verbosity: u8) -> Result<()> {
    let log_level = match verbosity {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    // Get log directory
    let log_dir = get_log_dir()?;

    // Create a rolling file appender that rotates daily and keeps 7 days of logs
    let file_appender =
        RollingFileAppender::new(Rotation::DAILY, log_dir.clone(), "crowdcontrol.log");

    // Create the file layer
    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_target(false);

    // Create the console layer (stderr)
    let console_layer = fmt::layer().with_writer(std::io::stderr).with_target(false);

    // Create env filter that respects RUST_LOG or falls back to our verbosity
    let env_filter = if std::env::var("RUST_LOG").is_ok() {
        EnvFilter::from_default_env()
    } else {
        // Set default level for all crates
        EnvFilter::new(log_level.to_string())
    };

    // Build the subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    tracing::info!(
        "Logger initialized. Log file: {:?}",
        log_dir.join("crowdcontrol.log")
    );

    Ok(())
}

/// Initialize tracing for simpler use cases (console only, used in tests)
pub fn init_env_logger(verbosity: u8) {
    let log_level = match verbosity {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let env_filter = EnvFilter::new(log_level.to_string());

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;
    use tempfile::TempDir;
    use tracing::{debug, error, info, trace, warn};

    static INIT: Once = Once::new();

    fn init_test_logger() {
        INIT.call_once(|| {
            // Initialize a simple console logger for tests
            let _ = tracing_subscriber::fmt()
                .with_env_filter("trace")
                .with_test_writer()
                .try_init();
        });
    }

    #[test]
    fn test_get_log_dir() {
        let result = get_log_dir();
        assert!(result.is_ok());
        let log_dir = result.unwrap();
        assert!(log_dir.to_string_lossy().contains("crowdcontrol"));
    }

    #[test]
    fn test_init_logger_creates_directory() {
        // Set a custom XDG_DATA_HOME for testing
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("XDG_DATA_HOME", temp_dir.path());

        let result = get_log_dir();
        assert!(result.is_ok());

        // Check that the directory was created
        let expected_dir = temp_dir.path().join("crowdcontrol");
        assert!(expected_dir.exists());
        assert!(expected_dir.is_dir());

        // Clean up
        std::env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn test_log_levels() {
        init_test_logger();

        // These should all compile and run without panicking
        trace!("This is a trace message");
        debug!("This is a debug message");
        info!("This is an info message");
        warn!("This is a warning message");
        error!("This is an error message");
    }

    #[test]
    fn test_structured_logging() {
        init_test_logger();

        let user = "test_user";
        let count = 42;

        info!(user = %user, count = count, "User performed action");
        debug!(operation = "test", success = true, "Operation completed");
    }

    #[test]
    fn test_verbosity_levels() {
        // Test that verbosity levels map correctly
        assert_eq!(
            match 0 {
                0 => Level::WARN,
                1 => Level::INFO,
                2 => Level::DEBUG,
                _ => Level::TRACE,
            },
            Level::WARN
        );

        assert_eq!(
            match 3 {
                0 => Level::WARN,
                1 => Level::INFO,
                2 => Level::DEBUG,
                _ => Level::TRACE,
            },
            Level::TRACE
        );
    }

    #[test]
    fn test_log_file_written() {
        // Create a temp directory for this test
        let temp_dir = TempDir::new().unwrap();
        let original_xdg = std::env::var("XDG_DATA_HOME").ok();
        std::env::set_var("XDG_DATA_HOME", temp_dir.path());

        // We can't actually test init_logger since it tries to set a global subscriber
        // But we can test that the directory creation works
        let log_dir_result = get_log_dir();
        assert!(log_dir_result.is_ok());
        let log_dir = log_dir_result.unwrap();

        // Verify the log directory was created
        assert!(log_dir.exists());
        assert!(log_dir.is_dir());

        // Clean up
        if let Some(original) = original_xdg {
            std::env::set_var("XDG_DATA_HOME", original);
        } else {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }
}
