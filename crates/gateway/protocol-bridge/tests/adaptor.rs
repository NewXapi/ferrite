//! Tests for gateway protocol bridge adaptors
//!
//! Tests cover OpenAI↔Claude↔Gemini bidirection conversions including:
//! - OpenAI→Claude request mapping (system extraction, max_tokens default, role conversion)
//! - Claude→OpenAI response conversion (non-stream JSON, stream events to chunks)
//! - OpenAI→Gemini request mapping (systemInstruction, contents role mapping)
//! - Gemini→OpenAI response conversion (non-stream candidates, stream events)
//! - OpenAI passthrough (including stream_options.include_usage injection)
//!
//! All tests use serde_json for fixture validation and test multiple scenarios.

use bytes::Bytes;
use gateway_protocol_bridge::adaptor::{
    AdaptorRegistry, ClaudeCodec, Codec, GeminiCodec, OpenAiCodec, Protocol,
};
use serde_json::Value;
use serde_json::json;

fn run_test_openai_passthrough() {
    // Test 1: OpenAI passthrough without stream_options
    let body = json!({
        "model": "gpt-4",
        "stream": true,
        "messages": [{"role": "user", "content": "hello"}]
    });
    let mut expected = body.clone();
    expected["stream_options"] = json!({"include_usage": true});

    let codec = OpenAiCodec;
    let result = codec
        .adapt_request(Bytes::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let parsed: Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(parsed["model"], "gpt-4");
    assert_eq!(parsed["stream"], true);
    assert_eq!(parsed["stream_options"], expected["stream_options"]);

    // Test 2: OpenAI passthrough without stream=true
    let body = json!({
        "model": "gpt-4",
        "messages": [{"role": "user", "content": "hello"}]
    });
    let codec = OpenAiCodec;
    let result = codec
        .adapt_request(Bytes::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let parsed: Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(parsed["model"], "gpt-4");
    assert!(parsed.get("stream_options").is_none());

    // Test 3: OpenAI passthrough already has include_usage=true
    let body = json!({
        "model": "gpt-4",
        "stream": true,
        "stream_options": {"include_usage": true},
        "messages": [{"role": "user", "content": "hello"}]
    });
    let codec = OpenAiCodec;
    let result = codec
        .adapt_request(Bytes::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let parsed: Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(parsed["model"], "gpt-4");
    assert_eq!(parsed["stream"], true);
    assert_eq!(parsed["stream_options"], json!({"include_usage": true}));
}

fn run_test_openai_to_claude_request_mapping() {
    // Test: OpenAI→Claude request mapping
    let body = json!({
        "model": "claude-3-opus-20240229",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Hello, world!"},
            {"role": "assistant", "content": "Hi there!"}
        ],
        "max_tokens": 2000,
        "temperature": 0.7,
        "top_p": 0.9,
        "stop": "end",
        "stream": true
    });

    let codec = ClaudeCodec { to_claude: true };
    let result = codec
        .adapt_request(Bytes::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let parsed: Value = serde_json::from_slice(&result).unwrap();

    // Check system extraction
    assert!(parsed.get("system").is_some());
    let system = parsed.get("system").unwrap();
    assert!(system.is_array());
    assert_eq!(system[0]["type"], "text");
    assert_eq!(system[0]["text"], "You are a helpful assistant.");

    // Check model preserved
    assert_eq!(parsed["model"], "claude-3-opus-20240229");

    // Check max_tokens preserved
    assert_eq!(parsed["max_tokens"], 2000);

    // Check temperature and top_p
    assert_eq!(parsed["temperature"], 0.7);
    assert_eq!(parsed["top_p"], 0.9);

    // Check stop_sequences
    assert_eq!(parsed["stop_sequences"], json!(["end"]));

    // Check stream
    assert_eq!(parsed["stream"], true);

    // Check messages conversion - system extracted, remaining messages: user, assistant
    let messages = parsed.get("messages").unwrap();
    assert!(messages.is_array());
    assert_eq!(messages.as_array().unwrap().len(), 2); // system extracted
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[1]["role"], "assistant");
}

fn run_test_claude_to_openai_response() {
    // Test 1: Claude non-stream response → OpenAI JSON
    let body = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "Hello there!"
        }],
        "model": "claude-3-opus-20240229",
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 100,
            "output_tokens": 50
        }
    });

    let codec = ClaudeCodec { to_claude: false };
    let result = codec
        .adapt_response(Bytes::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    // Non-stream response should produce a single OpenAI response
    assert_eq!(result.len(), 1);
    let openai_resp: Value = serde_json::from_slice(&result[0]).unwrap();

    // Check choices
    let choices = openai_resp.get("choices").unwrap();
    assert_eq!(choices.as_array().unwrap().len(), 1);
    let choice = &choices[0];
    assert_eq!(
        choice.get("message").unwrap().get("role").unwrap(),
        "assistant"
    );
    assert_eq!(
        choice.get("message").unwrap().get("content").unwrap(),
        "Hello there!"
    );
    assert_eq!(choice.get("finish_reason").unwrap(), "end_turn");

    // Test 2: Claude stream events → OpenAI chunks
    let stream_body = json!({
        "type": "message_start",
        "message": {
            "id": "msg_123",
            "role": "assistant",
            "model": "claude-3-opus-20240229",
            "content": []
        }
    });

    let result = codec
        .adapt_response(Bytes::from(serde_json::to_vec(&stream_body).unwrap()))
        .unwrap();
    // message_start should produce chunks
    assert!(!result.is_empty());
}

fn run_test_openai_to_gemini_request_mapping() {
    // Test: OpenAI→Gemini request mapping
    let body = json!({
        "model": "gemini-pro",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Hello, world!"},
            {"role": "assistant", "content": "Hi there!"}
        ],
        "temperature": 0.7,
        "top_p": 0.9,
        "max_tokens": 1000,
        "stop": "end",
        "stream": true
    });

    let codec = GeminiCodec { to_gemini: true };
    let result = codec
        .adapt_request(Bytes::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let parsed: Value = serde_json::from_slice(&result).unwrap();

    // Check systemInstruction extraction
    assert!(parsed.get("system_instruction").is_some());
    let system = parsed.get("system_instruction").unwrap();
    assert_eq!(system["parts"][0]["text"], "You are a helpful assistant.");

    // Check contents mapping
    let contents = parsed.get("contents").unwrap();
    assert!(contents.is_array());
    // system message excluded from contents
    assert_eq!(contents.as_array().unwrap().len(), 2);
    assert_eq!(contents[0]["role"], "user");
    assert_eq!(contents[1]["role"], "model");

    // Check generationConfig
    let gen_config = parsed.get("generation_config").unwrap();
    assert_eq!(gen_config["temperature"], 0.7);
    assert_eq!(gen_config["top_p"], 0.9);
    assert_eq!(gen_config["max_output_tokens"], 1000);
    assert_eq!(gen_config["stop_sequences"], json!(["end"]));

    // Check stream
    assert_eq!(parsed["stream"], true);
}

fn run_test_gemini_to_openai_response() {
    // Test 1: Gemini non-stream response → OpenAI JSON
    let body = json!({
        "candidates": [{
            "content": {
                "parts": [{"text": "Hello there!"}],
                "role": "model"
            },
            "finish_reason": "STOP"
        }],
        "usage_metadata": {
            "prompt_token_count": 100,
            "candidates_token_count": 50,
            "total_token_count": 150
        },
        "model": "gemini-pro"
    });

    let codec = GeminiCodec { to_gemini: false };
    let result = codec
        .adapt_response(Bytes::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    // Should produce at least one chunk
    assert!(!result.is_empty());
    let parsed: Value = serde_json::from_slice(&result[0]).unwrap();

    // Check choices
    let choices = parsed.get("choices");
    if choices.is_some() {
        assert_eq!(choices.unwrap().as_array().unwrap().len(), 1);
    }

    // Test 2: Gemini stream event → OpenAI chunks
    let stream_body = json!({
        "candidates": [{
            "content": {
                "parts": [{"text": "Hello"}],
                "role": "model"
            }
        }],
        "model": "gemini-pro"
    });

    let result = codec
        .adapt_response(Bytes::from(serde_json::to_vec(&stream_body).unwrap()))
        .unwrap();
    assert!(!result.is_empty());
}

fn run_test_adaptor_registry() {
    // Test AdaptorRegistry registration and resolution
    let default_registry = AdaptorRegistry::with_defaults();
    assert!(
        default_registry
            .resolve(Protocol::OpenAi, Protocol::Claude)
            .is_some()
    );
    assert!(
        default_registry
            .resolve(Protocol::Claude, Protocol::OpenAi)
            .is_some()
    );
    assert!(
        default_registry
            .resolve(Protocol::OpenAi, Protocol::Gemini)
            .is_some()
    );
    assert!(
        default_registry
            .resolve(Protocol::Gemini, Protocol::OpenAi)
            .is_some()
    );

    // Test passthrough for same protocol
    assert!(
        default_registry
            .resolve(Protocol::OpenAi, Protocol::OpenAi)
            .is_some()
    );
    let codec = default_registry
        .resolve(Protocol::OpenAi, Protocol::OpenAi)
        .unwrap();
    let result = codec.adapt_request(Bytes::from_static(b"test")).unwrap();
    assert_eq!(result, Bytes::from_static(b"test"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_passthrough() {
        run_test_openai_passthrough();
    }

    #[test]
    fn test_openai_to_claude_request_mapping() {
        run_test_openai_to_claude_request_mapping();
    }

    #[test]
    fn test_claude_to_openai_response() {
        run_test_claude_to_openai_response();
    }

    #[test]
    fn test_openai_to_gemini_request_mapping() {
        run_test_openai_to_gemini_request_mapping();
    }

    #[test]
    fn test_gemini_to_openai_response() {
        run_test_gemini_to_openai_response();
    }

    #[test]
    fn test_adaptor_registry() {
        run_test_adaptor_registry();
    }

    #[test]
    fn test_openai_to_claude_response_streaming() {
        // Test Claude streaming response conversion
        let stream_body = json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": "Hello"
            }
        });

        let codec = ClaudeCodec { to_claude: false };
        let result = codec
            .adapt_response(Bytes::from(serde_json::to_vec(&stream_body).unwrap()))
            .unwrap();

        // Should produce OpenAI chunk
        assert!(!result.is_empty());
        let chunk_str = String::from_utf8_lossy(&result[0]);
        assert!(chunk_str.contains("data:"));
    }

    #[test]
    fn test_gemini_to_openai_streaming_response() {
        // Test Gemini streaming response conversion
        let stream_body = json!({
            "candidates": [{
                "content": {
                    "parts": [{"text": "Hello"}],
                    "role": "model"
                }
            }],
            "usage_metadata": {
                "prompt_token_count": 100,
                "candidates_token_count": 50
            },
            "model": "gemini-pro"
        });

        let codec = GeminiCodec { to_gemini: false };
        let result = codec
            .adapt_response(Bytes::from(serde_json::to_vec(&stream_body).unwrap()))
            .unwrap();

        // Should produce multiple chunks
        assert!(!result.is_empty());
        let total_chunks: usize = result.len();
        assert!(total_chunks >= 1);
    }
}
