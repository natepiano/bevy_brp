//! Shared HTTP client for BRP operations with connection pooling
//!
//! This module provides a singleton HTTP client that reuses connections
//! to prevent resource exhaustion under concurrent load.

use std::sync::LazyLock;
use std::time::Duration;

use reqwest::Client;

/// Shared HTTP client instance with optimized connection pooling
///
/// This client is configured for BRP usage patterns:
/// - Connection pooling enabled for localhost connections
/// - Reasonable timeouts for local services
/// - Connection keep-alive for reduced overhead
static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(10)
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| Client::new())
});

/// Get the shared HTTP client instance
///
/// This returns a reference to a singleton `reqwest::Client` that:
/// - Reuses TCP connections via connection pooling
/// - Prevents resource exhaustion under concurrent load
/// - Is optimized for local BRP server communication
pub fn get_client() -> &'static Client {
    &HTTP_CLIENT
}

/// Create a new HTTP client with a custom timeout for watch operations
///
/// This creates a new `reqwest::Client` with:
/// - Custom timeout duration (or no timeout if None)
/// - Connection pooling optimized for single-host usage
/// - Suitable for long-running SSE connections
///
/// # Arguments
/// * `timeout_seconds` - Optional timeout in seconds (None = default 30s, Some(0) = never timeout)
pub fn create_watch_client(timeout_seconds: Option<u32>) -> Client {
    match timeout_seconds {
        Some(0) => {
            // 0 = never timeout (no timeout set on client)
            Client::builder()
                .pool_idle_timeout(Duration::from_secs(60))
                .pool_max_idle_per_host(5)
                .connect_timeout(Duration::from_secs(5))
                // No timeout() call means no request timeout
                .build()
                .unwrap_or_else(|_| Client::new())
        }
        Some(seconds) => {
            // Specific timeout in seconds
            Client::builder()
                .pool_idle_timeout(Duration::from_secs(60))
                .pool_max_idle_per_host(5)
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(u64::from(seconds)))
                .build()
                .unwrap_or_else(|_| Client::new())
        }
        None => {
            // Default to 30 seconds if not specified
            get_client().clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_singleton() {
        let client1 = get_client();
        let client2 = get_client();

        // Both references should point to the same instance
        assert!(std::ptr::eq(client1, client2));
    }
}
