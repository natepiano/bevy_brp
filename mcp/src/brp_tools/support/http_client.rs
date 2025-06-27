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
