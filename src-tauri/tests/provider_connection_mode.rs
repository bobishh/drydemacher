use ecky_cad_lib::mcp::server::{connection_uses_embedded_mcp, provider_bound_endpoint};

#[test]
fn provider_adapter_and_mcp_routes_keep_embedded_tools_available() {
    assert!(connection_uses_embedded_mcp(Some("mcp")));
    assert!(connection_uses_embedded_mcp(Some("provider:codex")));
    assert!(connection_uses_embedded_mcp(Some("provider:agy")));
    assert!(connection_uses_embedded_mcp(Some("provider:claude-code")));
    assert!(!connection_uses_embedded_mcp(Some("api_key")));
    assert!(!connection_uses_embedded_mcp(None));
}

#[test]
fn provider_endpoint_carries_exact_prebound_thread() {
    assert_eq!(
        provider_bound_endpoint("http://127.0.0.1:39249/mcp", "thread 1/ä"),
        "http://127.0.0.1:39249/mcp?providerThreadId=thread%201%2F%C3%A4"
    );
}
