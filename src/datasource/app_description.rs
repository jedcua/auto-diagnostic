use crate::lib::config::AppDescConfig;
use crate::lib::prompt::PromptData;

pub fn fetch_data(config: &AppDescConfig) -> PromptData {
    PromptData {
        description: vec![
            "Information: [App Description]".to_string(),
            config.description.clone()
        ],
        data: None
    }
}
