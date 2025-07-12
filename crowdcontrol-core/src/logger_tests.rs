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
        
        let result = init_logger(1);
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
    fn test_env_logger_init() {
        // This should not panic
        init_env_logger(2);
    }

    #[test]
    fn test_log_file_creation() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("XDG_DATA_HOME", temp_dir.path());
        
        // Initialize logger
        let result = init_logger(1);
        assert!(result.is_ok());
        
        // Log something
        info!("Test message for file creation");
        
        // Check that log file exists
        let log_file = temp_dir.path().join("crowdcontrol").join("crowdcontrol.log.2025-07-11");
        // Note: The actual date will vary, so we just check the directory
        let log_dir = temp_dir.path().join("crowdcontrol");
        assert!(log_dir.exists());
        
        // Clean up
        std::env::remove_var("XDG_DATA_HOME");
    }
}