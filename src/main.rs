mod datasource {
    pub mod app_description;
    pub mod cloudwatch_log_insight;
    pub mod cloudwatch_metric;
    pub mod ec2;
    pub mod rds;
    pub mod ds;
}
mod lib {
    pub mod args;
    pub mod config;
    pub mod context;
    pub mod prompt;
    pub mod openai;
}

use crate::lib::config::Config;
use crate::lib::context::build_context;
use crate::lib::openai::OpenAiChatInput;
use crate::lib::{args, openai, prompt};
use clap::Parser;
use std::error::Error;
use async_openai::Client;
use tokio::fs;

const BANNER : &str = "
███╗     █████╗               ██████╗     ███╗
██╔╝    ██╔══██╗              ██╔══██╗    ╚██║
██║     ███████║    █████╗    ██║  ██║     ██║
██║     ██╔══██║    ╚════╝    ██║  ██║     ██║
███╗    ██║  ██║              ██████╔╝    ███║
╚══╝    ╚═╝  ╚═╝              ╚═════╝     ╚══╝
- auto-diagnostic {x.y.z} | Written by Jed Cua -";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = args::Args::parse();
    let toml_content = fs::read_to_string(&args.file).await?;
    let config: Config = toml::from_str(&toml_content)?;

    let context = build_context(args, config)?;

    let banner = BANNER.replace("{x.y.z}", env!("CARGO_PKG_VERSION"));
    println!("{banner}");

    let prompt_data = prompt::build_prompt_data(&context).await?;

    if context.print_prompt_data {
        println!("\n{prompt_data}\n");
    }

    if !context.dry_run {
        let client = Client::new();
        openai::send_request(client, &context, OpenAiChatInput {
            model: context.open_ai_model.clone(),
            max_tokens: context.open_ai_max_token,
            system_prompt: prompt::INSTRUCTION.to_string(),
            user_prompt: prompt_data
        }).await?;
    }

    Ok(())
}
