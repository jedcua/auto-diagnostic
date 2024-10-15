use crate::lib::context::AppContext;
use async_openai::config::OpenAIConfig;
use async_openai::types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, ChatCompletionResponseStream, CreateChatCompletionRequest, CreateChatCompletionRequestArgs};
use async_openai::Client;
use futures::StreamExt;
use std::env::VarError::NotPresent;
use std::error::Error;
use std::io::{stdout, Write};

#[derive(Default)]
pub struct OpenAiChatInput {
    pub model: String,
    pub max_tokens: u32,
    pub system_prompt: String,
    pub user_prompt: String
}

const OPENAI_API_KEY: &str = "OPENAI_API_KEY";

pub trait OpenAiClient {
    async fn create_stream(&self, request: CreateChatCompletionRequest) -> Result<ChatCompletionResponseStream, Box<dyn Error>>;
}

impl OpenAiClient for Client<OpenAIConfig> {
    async fn create_stream(&self, request: CreateChatCompletionRequest) -> Result<ChatCompletionResponseStream, Box<dyn Error>> {
        Ok(self.chat().create_stream(request).await?)
    }
}

pub async fn send_request(client: impl OpenAiClient, context: &AppContext, input: OpenAiChatInput) -> Result<String, Box<dyn Error>> {
    if let Err(NotPresent) = std::env::var(OPENAI_API_KEY) {
        let api_key = context.open_ai_api_key
            .clone()
            .expect(format!("{OPENAI_API_KEY} variable is not set").as_str());
        std::env::set_var(OPENAI_API_KEY, api_key);
    }

    let openai_request = CreateChatCompletionRequestArgs::default()
        .model(input.model)
        .max_tokens(input.max_tokens)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(input.system_prompt)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(input.user_prompt)
                .build()?
                .into()
        ])
        .build()?;

    let mut stream = client.create_stream(openai_request).await?;
    let mut output = String::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                response.choices.iter().for_each(|chat_choice| {
                    if let Some(ref content) = chat_choice.delta.content {
                        output.push_str(content);
                        print!("{content}");
                    }
                });
            }
            Err(err) => {
                output.push_str(format!("{err}").as_str());
                print!("error: {err}");
            }
        }
        stdout().flush()?;
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use crate::lib::context::AppContext;
    use crate::lib::openai::{send_request, OpenAiChatInput, OpenAiClient, OPENAI_API_KEY};
    use async_openai::types::{ChatChoiceStream, ChatCompletionResponseStream, ChatCompletionStreamResponseDelta, CreateChatCompletionRequest, CreateChatCompletionStreamResponse};
    use futures::stream;
    use std::error::Error;

    struct MockOpenAiClient {}

    impl MockOpenAiClient {
        #[allow(deprecated)]
        fn build_stream_response(content: &str) -> CreateChatCompletionStreamResponse {
            CreateChatCompletionStreamResponse {
                id: "id".to_string(),
                choices: vec![
                    ChatChoiceStream {
                        index: 0,
                        delta: ChatCompletionStreamResponseDelta {
                            content: Some(content.to_string()),
                            function_call: None,
                            tool_calls: None,
                            role: None,
                        },
                        finish_reason: None,
                        logprobs: None,
                    }
                ],
                created: 0,
                model: "gpt-40".to_string(),
                system_fingerprint: None,
                object: "chat.completion.chunk".to_string(),
                usage: None
            }
        }
    }

    impl OpenAiClient for MockOpenAiClient {
        async fn create_stream(&self, _: CreateChatCompletionRequest) -> Result<ChatCompletionResponseStream, Box<dyn Error>> {
            let stream = stream::iter(vec![
                Ok(Self::build_stream_response("The ")),
                Ok(Self::build_stream_response("quick ")),
                Ok(Self::build_stream_response("brown ")),
                Ok(Self::build_stream_response("fox ")),
                Ok(Self::build_stream_response("jumps ")),
                Ok(Self::build_stream_response("over ")),
                Ok(Self::build_stream_response("the ")),
                Ok(Self::build_stream_response("lazy ")),
                Ok(Self::build_stream_response("dog.")),
            ]);

            Ok(Box::pin(stream) as ChatCompletionResponseStream)
        }
    }

    #[tokio::test]
    async fn test_send_request() {
        let client = MockOpenAiClient {};
        let context = AppContext {
            open_ai_api_key: Some("openai-api-key-123456789".to_string()),
            ..AppContext::default()
        };
        let input = OpenAiChatInput::default();

        std::env::remove_var(OPENAI_API_KEY);
        let output = send_request(client, &context, input).await.expect("Should be able to send request");

        assert_eq!(std::env::var(OPENAI_API_KEY).ok(), context.open_ai_api_key);
        assert_eq!(output, "The quick brown fox jumps over the lazy dog.");
    }
}