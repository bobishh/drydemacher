#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <thread-id> <ecky-source-file> [mcp-url]" >&2
  echo "Example: $0 thread-123 model-runtime/examples/film-scanning-adapter-helicoid.ecky http://127.0.0.1:39249/mcp" >&2
  exit 1
fi

THREAD_ID="$1"
SOURCE_FILE="$2"
MCP_URL="${3:-http://127.0.0.1:39249/mcp}"
GEOMETRY_BACKEND="${GEOMETRY_BACKEND:-mesh}"

if [[ ! -f "$SOURCE_FILE" ]]; then
  echo "Source file not found: $SOURCE_FILE" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl required" >&2
  exit 1
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "jq required" >&2
  exit 1
fi

"$ROOT/scripts/guard_no_direct_db_write.sh" || {
  echo "Direct DB write guard failed." >&2
  exit 1
}

SOURCE_CODE="$(cat "$SOURCE_FILE")"
SOURCE_JSON="$(printf '%s' "$SOURCE_CODE" | jq -Rs .)"

init_headers="$(mktemp)"
init_body="$(mktemp)"
mcp_context_file="$(mktemp)"
trap 'rm -f "$init_headers" "$init_body" "$mcp_context_file"' EXIT

MCP_PROTOCOL_VERSION="2026-07-28"
MCP_MODE="modern"
SESSION_ID=""

curl -sS \
  -D "$init_headers" \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H "MCP-Protocol-Version: $MCP_PROTOCOL_VERSION" \
  -H 'Mcp-Method: server/discover' \
  -d '{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"mcp-smoke","version":"2.0"},"io.modelcontextprotocol/clientCapabilities":{}}}}' \
  "$MCP_URL" >"$init_body"

if ! jq -e '.result.supportedVersions | index("2026-07-28") != null' "$init_body" >/dev/null 2>&1; then
  MCP_MODE="legacy"
  curl -sS \
    -D "$init_headers" \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"mcp-smoke","version":"2.0"}}}' \
    "$MCP_URL" >"$init_body"
  SESSION_ID="$(awk 'tolower($1) == "mcp-session-id:" {print $2}' "$init_headers" | tr -d '\r\n' | tail -n1)"
  if [[ -z "$SESSION_ID" ]]; then
    echo "Neither modern discovery nor legacy initialize succeeded." >&2
    cat "$init_body" >&2
    exit 1
  fi
fi

rpc_call() {
  local id="$1"
  local tool_name="$2"
  local arguments_json="$3"
  local response
  if [[ "$MCP_MODE" == "modern" ]]; then
    local context_id
    local request_body
    context_id="$(tr -d '\r\n' <"$mcp_context_file")"
    arguments_json="$(jq -c --arg contextId "$context_id" \
      'if $contextId == "" then . else . + {eckyContextId:$contextId} end' \
      <<<"$arguments_json")"
    request_body="$(jq -cn \
      --argjson id "$id" \
      --arg name "$tool_name" \
      --argjson arguments "$arguments_json" \
      --arg protocolVersion "$MCP_PROTOCOL_VERSION" \
      '{jsonrpc:"2.0", id:$id, method:"tools/call", params:{name:$name, arguments:$arguments, _meta:{"io.modelcontextprotocol/protocolVersion":$protocolVersion, "io.modelcontextprotocol/clientInfo":{name:"mcp-smoke", version:"2.0"}, "io.modelcontextprotocol/clientCapabilities":{}}}}')"
    response="$(curl -sS \
      -H 'Content-Type: application/json' \
      -H 'Accept: application/json, text/event-stream' \
      -H "MCP-Protocol-Version: $MCP_PROTOCOL_VERSION" \
      -H 'Mcp-Method: tools/call' \
      -H "Mcp-Name: $tool_name" \
      -d "$request_body" \
      "$MCP_URL")"
    jq -r '.result.structuredContent.eckyContextId // empty' <<<"$response" >"$mcp_context_file"
  else
    response="$(curl -sS \
      -H 'Content-Type: application/json' \
      -H "Mcp-Session-Id: $SESSION_ID" \
      -d "{\"jsonrpc\":\"2.0\",\"id\":$id,\"method\":\"tools/call\",\"params\":{\"name\":\"$tool_name\",\"arguments\":$arguments_json}}" \
      "$MCP_URL")"
  fi
  printf '%s\n' "$response"
}

borrow_resp="$(rpc_call 2 thread_borrow "{\"threadId\":$(jq -Rn --arg v "$THREAD_ID" '$v')}" )"
echo "$borrow_resp" | jq -e 'if .error then false else true end' >/dev/null

preview_args="$(jq -cn \
  --argjson macroCode "$SOURCE_JSON" \
  --arg backend "$GEOMETRY_BACKEND" \
  '{macroCode:$macroCode, geometryBackend:$backend}')"
preview_resp="$(rpc_call 3 macro_preview_render "$preview_args")"
echo "$preview_resp" | jq -e 'if .error then false else true end' >/dev/null

thread_id="$(echo "$preview_resp" | jq -er '.result.structuredContent.threadId')"
message_id="$(echo "$preview_resp" | jq -er '.result.structuredContent.messageId')"
model_id="$(echo "$preview_resp" | jq -er '.result.structuredContent.artifactBundle.modelId')"
verify_args="$(jq -cn \
  --arg threadId "$thread_id" \
  --arg messageId "$message_id" \
  --arg modelId "$model_id" \
  '{threadId:$threadId, messageId:$messageId, modelId:$modelId}')"
verify_resp="$(rpc_call 4 verify_generated_model "$verify_args")"
echo "$verify_resp" | jq -e 'if .error then false else true end' >/dev/null

echo "Preview response:"
echo "$preview_resp" | jq '{result: .result, error: .error}'
echo
echo "Verification response:"
echo "$verify_resp" | jq '{result: .result, error: .error}'
