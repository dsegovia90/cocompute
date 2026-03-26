use bitcode::{Decode, Encode};
use common::protocols::{
    ChatStreamFrame, Metering, Request, Response,
    chat::{ChatMessage, ChatRequest, ChatResponse},
    embeddings::{EmbeddingsRequest, EmbeddingsResponse},
    registry::{Capabilities, ModelInfo, RegistryRequest},
};

/// Helper: encode then decode, verify roundtrip.
fn roundtrip<T: Encode + for<'a> Decode<'a> + std::fmt::Debug>(value: &T) -> T {
    let encoded = bitcode::encode(value);
    bitcode::decode(&encoded).expect("decode failed")
}

#[test]
fn embeddings_request_roundtrip() {
    let req = EmbeddingsRequest {
        model: "mxbai-embed-large:latest".into(),
        text: "hello world".into(),
    };
    let decoded: EmbeddingsRequest = roundtrip(&req);
    assert_eq!(decoded.model, "mxbai-embed-large:latest");
    assert_eq!(decoded.text, "hello world");
}

#[test]
fn embeddings_response_roundtrip() {
    let resp = EmbeddingsResponse::new(vec![0.1, 0.2, 0.3]);
    let decoded: EmbeddingsResponse = roundtrip(&resp);
    assert_eq!(decoded.embeddings, vec![0.1, 0.2, 0.3]);
}

#[test]
fn chat_request_roundtrip() {
    let req = ChatRequest {
        model: "llama3:latest".into(),
        messages: vec![
            ChatMessage { role: "system".into(), content: "You are helpful.".into() },
            ChatMessage { role: "user".into(), content: "Hello".into() },
        ],
        temperature: Some(0.7),
        stream: false,
    };
    let decoded: ChatRequest = roundtrip(&req);
    assert_eq!(decoded.model, "llama3:latest");
    assert_eq!(decoded.messages.len(), 2);
    assert_eq!(decoded.messages[0].role, "system");
    assert_eq!(decoded.messages[1].content, "Hello");
    assert_eq!(decoded.temperature, Some(0.7));
}

#[test]
fn chat_request_no_temperature_roundtrip() {
    let req = ChatRequest {
        model: "llama3:latest".into(),
        messages: vec![ChatMessage { role: "user".into(), content: "Hi".into() }],
        temperature: None,
        stream: false,
    };
    let decoded: ChatRequest = roundtrip(&req);
    assert_eq!(decoded.temperature, None);
}

#[test]
fn chat_response_roundtrip() {
    let resp = ChatResponse {
        message: ChatMessage { role: "assistant".into(), content: "Hello!".into() },
    };
    let decoded: ChatResponse = roundtrip(&resp);
    assert_eq!(decoded.message.role, "assistant");
    assert_eq!(decoded.message.content, "Hello!");
}

#[test]
fn registry_register_roundtrip() {
    let req = RegistryRequest::Register(Capabilities {
        models: vec![
            ModelInfo {
                name: "mxbai-embed-large:latest".into(),
                quantization: "f16".into(),
                vram_mb: 2048,
                ram_mb: 4096,
            },
            ModelInfo {
                name: "llama3:latest".into(),
                quantization: "q4_0".into(),
                vram_mb: 8192,
                ram_mb: 16384,
            },
        ],
    });
    let decoded: RegistryRequest = roundtrip(&req);
    match decoded {
        RegistryRequest::Register(caps) => {
            assert_eq!(caps.models.len(), 2);
            assert_eq!(caps.models[0].name, "mxbai-embed-large:latest");
            assert_eq!(caps.models[1].vram_mb, 8192);
        }
        _ => panic!("expected Register variant"),
    }
}

#[test]
fn registry_heartbeat_roundtrip() {
    let req = RegistryRequest::Heartbeat;
    let decoded: RegistryRequest = roundtrip(&req);
    assert!(matches!(decoded, RegistryRequest::Heartbeat));
}

#[test]
fn metering_roundtrip() {
    let m = Metering {
        prompt_tokens: 100,
        completion_tokens: 50,
        compute_ms: 1234,
    };
    let decoded: Metering = roundtrip(&m);
    assert_eq!(decoded.prompt_tokens, 100);
    assert_eq!(decoded.completion_tokens, 50);
    assert_eq!(decoded.compute_ms, 1234);
}

#[test]
fn request_enum_embeddings_roundtrip() {
    let req = Request::Embeddings(EmbeddingsRequest {
        model: "test".into(),
        text: "data".into(),
    });
    let decoded: Request = roundtrip(&req);
    match decoded {
        Request::Embeddings(e) => {
            assert_eq!(e.model, "test");
            assert_eq!(e.text, "data");
        }
        _ => panic!("expected Embeddings variant"),
    }
}

#[test]
fn request_enum_chat_roundtrip() {
    let req = Request::Chat(ChatRequest {
        model: "llama3:latest".into(),
        messages: vec![ChatMessage { role: "user".into(), content: "Hi".into() }],
        temperature: None,
        stream: false,
    });
    let decoded: Request = roundtrip(&req);
    assert!(matches!(decoded, Request::Chat(_)));
}

#[test]
fn request_enum_registry_roundtrip() {
    let req = Request::Registry(RegistryRequest::Heartbeat);
    let decoded: Request = roundtrip(&req);
    assert!(matches!(decoded, Request::Registry(RegistryRequest::Heartbeat)));
}

#[test]
fn response_embeddings_with_metering_roundtrip() {
    let resp = Response::Embeddings {
        result: EmbeddingsResponse::new(vec![1.0, 2.0]),
        metering: Metering {
            prompt_tokens: 10,
            completion_tokens: 0,
            compute_ms: 500,
        },
    };
    let decoded: Response = roundtrip(&resp);
    match decoded {
        Response::Embeddings { result, metering } => {
            assert_eq!(result.embeddings, vec![1.0, 2.0]);
            assert_eq!(metering.prompt_tokens, 10);
            assert_eq!(metering.compute_ms, 500);
        }
        _ => panic!("expected Embeddings variant"),
    }
}

#[test]
fn response_chat_with_metering_roundtrip() {
    let resp = Response::Chat {
        result: ChatResponse {
            message: ChatMessage { role: "assistant".into(), content: "Hi!".into() },
        },
        metering: Metering {
            prompt_tokens: 20,
            completion_tokens: 5,
            compute_ms: 1000,
        },
    };
    let decoded: Response = roundtrip(&resp);
    match decoded {
        Response::Chat { result, metering } => {
            assert_eq!(result.message.content, "Hi!");
            assert_eq!(metering.completion_tokens, 5);
        }
        _ => panic!("expected Chat variant"),
    }
}

#[test]
fn max_message_size_constant_is_16mb() {
    assert_eq!(common::helpers::MAX_MESSAGE_SIZE, 16 * 1024 * 1024);
}

#[test]
fn empty_embeddings_roundtrip() {
    let resp = EmbeddingsResponse::new(vec![]);
    let decoded: EmbeddingsResponse = roundtrip(&resp);
    assert!(decoded.embeddings.is_empty());
}

#[test]
fn large_embedding_vector_roundtrip() {
    let large_vec: Vec<f32> = (0..4096).map(|i| i as f32 * 0.001).collect();
    let resp = EmbeddingsResponse::new(large_vec.clone());
    let decoded: EmbeddingsResponse = roundtrip(&resp);
    assert_eq!(decoded.embeddings.len(), 4096);
    assert_eq!(decoded.embeddings[0], 0.0);
    assert!((decoded.embeddings[4095] - 4.095).abs() < 0.001);
}

#[test]
fn chat_request_stream_true_roundtrip() {
    let req = ChatRequest {
        model: "llama3:latest".into(),
        messages: vec![ChatMessage { role: "user".into(), content: "Hi".into() }],
        temperature: None,
        stream: true,
    };
    let decoded: ChatRequest = roundtrip(&req);
    assert!(decoded.stream);
}

#[test]
fn response_chat_stream_start_roundtrip() {
    let resp = Response::ChatStreamStart;
    let decoded: Response = roundtrip(&resp);
    assert!(matches!(decoded, Response::ChatStreamStart));
}

#[test]
fn chat_stream_frame_delta_roundtrip() {
    let frame = ChatStreamFrame::Delta("Hello ".into());
    let decoded: ChatStreamFrame = roundtrip(&frame);
    match decoded {
        ChatStreamFrame::Delta(content) => assert_eq!(content, "Hello "),
        _ => panic!("expected Delta"),
    }
}

#[test]
fn chat_stream_frame_done_roundtrip() {
    let frame = ChatStreamFrame::Done(Metering {
        prompt_tokens: 15,
        completion_tokens: 42,
        compute_ms: 2000,
    });
    let decoded: ChatStreamFrame = roundtrip(&frame);
    match decoded {
        ChatStreamFrame::Done(m) => {
            assert_eq!(m.prompt_tokens, 15);
            assert_eq!(m.completion_tokens, 42);
            assert_eq!(m.compute_ms, 2000);
        }
        _ => panic!("expected Done"),
    }
}

#[test]
fn chat_stream_frame_empty_delta_roundtrip() {
    let frame = ChatStreamFrame::Delta(String::new());
    let decoded: ChatStreamFrame = roundtrip(&frame);
    match decoded {
        ChatStreamFrame::Delta(content) => assert!(content.is_empty()),
        _ => panic!("expected Delta"),
    }
}
