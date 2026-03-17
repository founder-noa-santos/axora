//! IPC Command Handlers for AXORA Desktop Application
//!
//! This module contains all Tauri IPC commands that can be invoked from the frontend.
//! Commands follow Tauri v2 conventions and include comprehensive error handling.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// Application metadata returned by `get_app_info`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    /// Application name
    pub name: String,
    /// Application version (semver)
    pub version: String,
    /// Application identifier
    pub identifier: String,
    /// Build timestamp (ISO 8601)
    pub build_time: String,
}

/// Ping command - basic connectivity test
///
/// # Returns
/// * `String` - Always returns "pong"
///
/// # Example (frontend)
/// ```typescript
/// const response = await invoke('ping');
/// console.log(response); // "pong"
/// ```
#[tauri::command]
pub fn ping() -> Result<String, String> {
    Ok("pong".to_string())
}

/// Get application version from Cargo.toml
///
/// # Returns
/// * `String` - Semantic version string (e.g., "0.1.0")
///
/// # Errors
/// Returns error if version cannot be read from package metadata
///
/// # Example (frontend)
/// ```typescript
/// const version = await invoke('get_version');
/// console.log(version); // "0.1.0"
/// ```
#[tauri::command]
pub fn get_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

/// Get comprehensive application information
///
/// # Returns
/// * `AppInfo` - Struct containing name, version, identifier, and build time
///
/// # Example (frontend)
/// ```typescript
/// const info = await invoke('get_app_info');
/// console.log(info.name); // "axora"
/// console.log(info.version); // "0.1.0"
/// ```
#[tauri::command]
pub fn get_app_info() -> Result<AppInfo, String> {
    Ok(AppInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        identifier: env!("CARGO_PKG_REPOSITORY")
            .unwrap_or("dev.axora.app")
            .to_string(),
        build_time: chrono_lite_timestamp(),
    })
}

/// Generate a lightweight timestamp for build time
/// Uses compile-time environment variables when available
fn chrono_lite_timestamp() -> String {
    // Try to get build timestamp from environment
    // In production, this would be set by the build system
    option_env!("BUILD_TIMESTAMP")
        .unwrap_or(env!("CARGO_PKG_VERSION"))
        .to_string()
}

/// Emit a custom event to the frontend
///
/// # Arguments
/// * `app` - Tauri app handle (automatically injected)
/// * `event` - Event name to emit
/// * `payload` - Event payload (must be JSON serializable)
///
/// # Example (frontend)
/// ```typescript
/// listen('custom-event', (event) => {
///   console.log('Received:', event.payload);
/// });
/// await invoke('emit_event', { event: 'custom-event', payload: { data: 'test' } });
/// ```
#[tauri::command]
pub fn emit_event(app: AppHandle, event: String, payload: serde_json::Value) -> Result<(), String> {
    app.emit(&event, payload)
        .map_err(|e| format!("Failed to emit event: {}", e))
}

/// Log a message to the native console
///
/// # Arguments
/// * `level` - Log level (debug, info, warn, error)
/// * `message` - Message to log
///
/// # Example (frontend)
/// ```typescript
/// await invoke('log_message', { level: 'info', message: 'User action performed' });
/// ```
#[tauri::command]
pub fn log_message(level: String, message: String) -> Result<(), String> {
    match level.as_str() {
        "debug" => log::debug!("{}", message),
        "info" => log::info!("{}", message),
        "warn" => log::warn!("{}", message),
        "error" => log::error!("{}", message),
        _ => log::info!("{}", message),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_returns_pong() {
        let result = ping();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "pong");
    }

    #[test]
    fn test_get_version_returns_valid_semver() {
        let result = get_version();
        assert!(result.is_ok());
        let version = result.unwrap();
        
        // Basic semver validation (major.minor.patch)
        let parts: Vec<&str> = version.split('.').collect();
        assert!(parts.len() >= 2, "Version should have at least major.minor");
        
        // Check that major and minor are numeric
        assert!(
            parts[0].parse::<u32>().is_ok(),
            "Major version should be numeric"
        );
        assert!(
            parts[1].parse::<u32>().is_ok(),
            "Minor version should be numeric"
        );
    }

    #[test]
    fn test_get_app_info_returns_metadata() {
        let result = get_app_info();
        assert!(result.is_ok());
        
        let info = result.unwrap();
        assert!(!info.name.is_empty(), "App name should not be empty");
        assert!(!info.version.is_empty(), "App version should not be empty");
        assert!(!info.identifier.is_empty(), "App identifier should not be empty");
        assert!(!info.build_time.is_empty(), "Build time should not be empty");
    }

    #[test]
    fn test_app_info_serialization() {
        let info = AppInfo {
            name: "test-app".to_string(),
            version: "1.0.0".to_string(),
            identifier: "dev.test.app".to_string(),
            build_time: "2024-01-01T00:00:00Z".to_string(),
        };

        // Test serialization
        let serialized = serde_json::to_string(&info);
        assert!(serialized.is_ok(), "AppInfo should serialize to JSON");

        // Test deserialization
        let json = r#"{
            "name": "test-app",
            "version": "1.0.0",
            "identifier": "dev.test.app",
            "build_time": "2024-01-01T00:00:00Z"
        }"#;
        let deserialized: Result<AppInfo, _> = serde_json::from_str(json);
        assert!(deserialized.is_ok(), "JSON should deserialize to AppInfo");
        
        let deserialized = deserialized.unwrap();
        assert_eq!(deserialized.name, "test-app");
        assert_eq!(deserialized.version, "1.0.0");
    }

    #[test]
    fn test_command_error_handling() {
        // Test that commands return Result types for proper error handling
        let ping_result = ping();
        assert!(ping_result.is_ok());

        let version_result = get_version();
        assert!(version_result.is_ok());

        let info_result = get_app_info();
        assert!(info_result.is_ok());

        // All commands should return Ok in normal conditions
        // Error cases would be tested with invalid inputs in integration tests
    }

    #[test]
    fn test_app_info_field_types() {
        let info = get_app_info().unwrap();

        // Verify all fields are properly populated strings
        assert!(
            info.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'),
            "App name should contain only valid characters"
        );

        // Version should follow semver pattern
        let version_parts: Vec<&str> = info.version.split('.').collect();
        assert!(
            version_parts.len() == 3,
            "Version should follow semver (major.minor.patch)"
        );

        for part in version_parts {
            assert!(
                part.parse::<u32>().is_ok(),
                "Each version part should be numeric"
            );
        }
    }
}
