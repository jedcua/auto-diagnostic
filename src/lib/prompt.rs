use crate::lib::context::AppContext;
use indicatif::{ProgressBar, ProgressStyle};
use std::error::Error;
use std::time::Duration;

#[derive(Debug)]
pub struct PromptData {
    pub description: Vec<String>,
    pub data: Option<String>
}

pub fn build_instruction() -> String {
    let instructions = [
        "You are an AWS diagnostic assistant.",
        "Perform a diagnosis using the provided information surrounded with `<data></data>` tags below.",
        "Include timestamps from important data, if necessary.",
        "Add visual elements such as graphs and tables drawn with ascii characters, if necessary",
        "Format your response in Markdown.",
        "Focus your report only on information that requires concern.",
    ];

    instructions.join("\n")
}

pub async fn build_prompt_data(context: &AppContext) -> Result<String, Box<dyn Error>> {
    let mut prompt = String::new();

    let progress_bar = initialize_progress_bar(context);
    for data_source in &context.data_sources {
        progress_bar.set_message(format!("{data_source}"));

        for prompt_data in data_source.fetch_data(context).await? {
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
        }

        progress_bar.inc(1);
    }

    progress_bar.finish_with_message("Fetched data sources");

    Ok(prompt)
}

fn initialize_progress_bar(context: &AppContext) -> ProgressBar {
    let progress_bar = ProgressBar::new(context.data_sources.len() as u64);
    progress_bar.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{msg:.green}] [{pos:.yellow}/{len:.yellow}]")
        .unwrap()
        .tick_strings(&["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘", "🌘"])
    );
    progress_bar.enable_steady_tick(Duration::from_millis(100));
    progress_bar
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::datasource::ds::DataSource::AppDescription;
    use crate::lib::config::AppDescConfig;

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
