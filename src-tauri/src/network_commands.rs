use crate::network_logger::{NetworkLog, NetworkLogger, NetworkStats, TimePeriod};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Shared state for the network logger
pub struct NetworkLoggerState {
    pub logger: Arc<Mutex<Option<NetworkLogger>>>,
}

/// Get network statistics for a specific time period
#[tauri::command]
pub async fn get_network_stats(
    period: TimePeriod,
    state: State<'_, NetworkLoggerState>,
) -> Result<NetworkStats, String> {
    let logger_guard = state.logger.lock().await;

    if let Some(logger) = logger_guard.as_ref() {
        logger
            .get_stats(period)
            .await
            .map_err(|e| format!("Failed to get network stats: {}", e))
    } else {
        Err("Network logger not initialized".to_string())
    }
}

/// Get daily network logs for a specific time period
#[tauri::command]
pub async fn get_network_logs(
    period: TimePeriod,
    state: State<'_, NetworkLoggerState>,
) -> Result<Vec<NetworkLog>, String> {
    let logger_guard = state.logger.lock().await;

    if let Some(logger) = logger_guard.as_ref() {
        logger
            .get_daily_logs(period)
            .await
            .map_err(|e| format!("Failed to get network logs: {}", e))
    } else {
        Err("Network logger not initialized".to_string())
    }
}

/// Cleanup old network logs (older than specified days)
#[tauri::command]
pub async fn cleanup_network_logs(
    days_to_keep: i64,
    state: State<'_, NetworkLoggerState>,
) -> Result<u64, String> {
    let logger_guard = state.logger.lock().await;

    if let Some(logger) = logger_guard.as_ref() {
        logger
            .cleanup_old_logs(days_to_keep)
            .await
            .map_err(|e| format!("Failed to cleanup logs: {}", e))
    } else {
        Err("Network logger not initialized".to_string())
    }
}
