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
    pub mod prompt;
    pub mod openai;
}

use crate::datasource::ds::DataSource;
use crate::datasource::ds::DataSource::{AppDescription, CloudwatchLogInsight, CloudwatchMetric, Ec2, Rds};
use crate::datasource::ec2;
use crate::lib::config::Config;
use crate::lib::openai::OpenAiChatInput;
use crate::lib::{args, openai, prompt};
use chrono_tz::Tz;
use clap::Parser;
use std::error::Error;
use tokio::fs;

const BANNER : &str = "
███╗     █████╗ ██╗   ██╗████████╗ ██████╗
██╔╝    ██╔══██╗██║   ██║╚══██╔══╝██╔═══██╗
██║     ███████║██║   ██║   ██║   ██║   ██║
██║     ██╔══██║██║   ██║   ██║   ██║   ██║
███╗    ██║  ██║╚██████╔╝   ██║   ╚██████╔╝
╚══╝    ╚═╝  ╚═╝ ╚═════╝    ╚═╝    ╚═════╝

██████╗ ██╗ █████╗  ██████╗ ███╗   ██╗ ██████╗ ███████╗████████╗██╗ ██████╗    ███╗
██╔══██╗██║██╔══██╗██╔════╝ ████╗  ██║██╔═══██╗██╔════╝╚══██╔══╝██║██╔════╝    ╚██║
██║  ██║██║███████║██║  ███╗██╔██╗ ██║██║   ██║███████╗   ██║   ██║██║          ██║
██║  ██║██║██╔══██║██║   ██║██║╚██╗██║██║   ██║╚════██║   ██║   ██║██║          ██║
██████╔╝██║██║  ██║╚██████╔╝██║ ╚████║╚██████╔╝███████║   ██║   ██║╚██████╗    ███║
╚═════╝ ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝ ╚══════╝   ╚═╝   ╚═╝ ╚═════╝    ╚══╝
================= version: {version} | written by: Jed Cua ================
";

struct AppContext {
    profile: String,
    start_time: i64,
    end_time: i64,
    time_zone: Tz,
    data_sources: Vec<DataSource>,
    open_ai_api_key: String,
    open_ai_model: String,
    open_ai_max_token: u32,
    print_prompt_data: bool,
    dry_run: bool
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let context = build_context().await?;

    let banner = BANNER.replace("{version}", env!("CARGO_PKG_VERSION"));
    println!("{banner}");

    let prompt_data = prompt::build_prompt_data(&context).await?;

    if context.print_prompt_data {
        println!("\n{prompt_data}\n");
    }

    if !context.dry_run {
        openai::send_request(&context, OpenAiChatInput {
            model: context.open_ai_model.clone(),
            max_tokens: context.open_ai_max_token,
            system_prompt: prompt::INSTRUCTION.to_string(),
            user_prompt: prompt_data
        }).await?;
    }

    Ok(())
}

async fn build_context() -> Result<AppContext, Box<dyn Error>> {
    let args = args::Args::parse();
    let toml_content = fs::read_to_string(&args.file).await?;
    let config: Config = toml::from_str(&toml_content)?;

    let time_zone = match config.general.time_zone {
        Some(tz) => tz.parse().expect("Unknown time zone"),
        None => Tz::UTC
    };

    let (start_time, end_time) = args::build_start_and_end(&args, time_zone)?;

    let mut data_sources: Vec<DataSource> = Vec::new();

    if let Some(configs) = config.app_description {
        for app_desc_config in configs {
            data_sources.push(AppDescription {
                order_no: app_desc_config.order_no,
                config: app_desc_config
            });
        }
    }

    if let Some(configs) = config.ec2 {
        for ec2_config in configs {
            data_sources.push(Ec2 {
                order_no: ec2_config.order_no,
                config: ec2_config
            });
        }
    }

    if let Some(configs) = config.rds {
        for rds_config in configs {
            data_sources.push(Rds {
                order_no: rds_config.order_no,
                config: rds_config
            });
        }
    }

    if let Some(configs) = config.cloudwatch_metric {
        for cloudwatch_config in configs {
            data_sources.push(CloudwatchMetric {
                order_no: cloudwatch_config.order_no,
                config: cloudwatch_config
            });
        }
    }

    if let Some(configs) = config.cloudwatch_log_insight {
        for cloudwatch_config in configs {
            data_sources.push(CloudwatchLogInsight {
                order_no: cloudwatch_config.order_no,
                config: cloudwatch_config
            });
        }
    }

    data_sources.sort();

    let context = AppContext {
        profile: String::from(&config.general.profile),
        start_time: start_time.as_millis() as i64,
        end_time: end_time.as_millis() as i64,
        time_zone,
        data_sources,
        open_ai_api_key: config.open_ai.api_key,
        open_ai_model: config.open_ai.model,
        open_ai_max_token: config.open_ai.max_token,
        print_prompt_data: args.print_prompt_data,
        dry_run: args.dry_run
    };

    Ok(context)
}
