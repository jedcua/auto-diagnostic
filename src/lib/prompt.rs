use crate::lib::context::AppContext;
use indicatif::{ProgressBar, ProgressStyle};
use std::error::Error;
use std::time::Duration;

#[derive(Debug)]
pub struct PromptData {
    pub description: Vec<String>,
    pub data: Option<String>
}

pub const INSTRUCTION: &str = concat!(
    "You are an AWS diagnostic assistant.\n",
    "You will be given pieces of information surrounded by `<data></data>` tags\n",
    "Use this information to perform a diagnosis.\n",
    "Base your diagnosis from the provided information only.\n",
    "Use all of the information provided in your diagnosis, but report only on information that needs immediate investigation/action.\n",
    "Keep your diagnosis precise and straight to the point.\n",
    "Format your response using Markdown.\n",
    "Listed below are the information you will use:\n",
);

pub async fn build_prompt_data(context: &AppContext) -> Result<String, Box<dyn Error>> {
    let mut prompt = String::new();

    let progress_bar = ProgressBar::new(context.data_sources.len() as u64);
    progress_bar.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{msg:.green}] [{pos:.yellow}/{len:.yellow}]")
        .unwrap()
        .tick_strings(&["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘", "🌘"])
    );
    progress_bar.enable_steady_tick(Duration::from_millis(100));

    for data_source in &context.data_sources {
        let prompt_data = data_source.fetch_data(context).await?;

        progress_bar.set_message(format!("{data_source}"));

        prompt.push_str("<data>\n");
        prompt.push_str(&prompt_data.description.join("\n"));
        prompt.push('\n');
        if let Some(data) = &prompt_data.data {
            prompt.push_str("Data:\n");
            prompt.push_str("```\n");
            prompt.push_str(data);
            prompt.push_str("```\n");
        }
        prompt.push_str("</data>\n");
        prompt.push('\n');

        progress_bar.inc(1);
    }

    progress_bar.finish_with_message("Fetched data sources");

    Ok(prompt)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lib::config::AppDescConfig;
    use crate::datasource::ds::DataSource::AppDescription;

    #[tokio::test]
    async fn should_build_prompt_data_correctly() {
        let context = AppContext {
            data_sources: vec![
                AppDescription {
                    config: AppDescConfig {
                        order_no: 1,
                        description: "This is an app description".to_string()
                    },
                }
            ],
            ..AppContext::default()
        };

        let prompt_data = build_prompt_data(&context).await.expect("Should build prompt data");
        let expected_prompt_data = "<data>\nInformation: [App Description]\nThis is an app description\n</data>\n\n".to_string();

        assert_eq!(prompt_data, expected_prompt_data);
    }
}
