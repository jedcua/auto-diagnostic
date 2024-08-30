use crate::lib::config::AppDescConfig;
use crate::lib::prompt::PromptData;
use std::error::Error;

pub fn fetch_data(config: &AppDescConfig) -> Result<PromptData, Box<dyn Error>> {
    Ok(PromptData {
        description: vec![
            "Information: [App Description]".to_string(),
            config.description.clone()
        ],
        data: None
    })
}
