use crate::AppContext;
use async_openai::types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs};
use async_openai::Client;
use futures::StreamExt;
use std::error::Error;
use std::io::{stdout, Write};

pub struct OpenAiChatInput {
    pub model: String,
    pub max_tokens: u32,
    pub system_prompt: String,
    pub user_prompt: String
}

pub async fn send_request(context: &AppContext, input: OpenAiChatInput) -> Result<(), Box<dyn Error>> {
    std::env::set_var("OPENAI_API_KEY", &context.open_ai_api_key);

    let client = Client::new();

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

    let mut stream = client.chat().create_stream(openai_request).await?;

    let mut lock = stdout().lock();
    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                response.choices.iter().for_each(|chat_choice| {
                    if let Some(ref content) = chat_choice.delta.content {
                        write!(lock, "{}", content).unwrap();
                    }
                });
            }
            Err(err) => {
                writeln!(lock, "error: {err}").unwrap();
            }
        }
        stdout().flush()?;
    }

    Ok(())
}