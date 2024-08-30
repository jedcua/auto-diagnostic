use crate::lib::config::RdsConfig;
use crate::lib::prompt::PromptData;
use crate::AppContext;
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_sdk_rds::types::DbInstance;
use aws_sdk_rds::Client;
use std::error::Error;

pub async fn fetch_data(context: &AppContext, config: &RdsConfig) -> Result<PromptData, Box<dyn Error>> {
    let client = init_client(&context.profile).await;

    let response = client.describe_db_instances()
        .send()
        .await?;

    for db_instance in response.db_instances.unwrap_or_default() {
        let name = db_instance.db_instance_identifier.clone().unwrap_or_default();
        if name == config.db_identifier {
            return Ok(PromptData {
                description: build_description(config, &db_instance),
                data: None
            })
        }
    }

    panic!("Unable to find DB instance with name: {}", config.db_identifier);
}

fn build_description(config: &RdsConfig, instance: &DbInstance) -> Vec<String> {
    vec![
        "Information: [RDS Instance]".to_string(),
        format!("DB identifier: [`{}`]", &config.db_identifier),
        format!("Class: [`{}`]", instance.db_instance_class().unwrap()),
        format!("Engine: [{} {}]", instance.engine().unwrap(), instance.engine_version().unwrap()),
        format!("Storage type: [{}]", instance.storage_type().unwrap()),
        format!("Status: [{}]", instance.db_instance_status().unwrap()),
        format!("Multi AZ: [{}]", instance.multi_az().unwrap()),
    ]
}

async fn init_client(aws_profile: &String) -> Client {
    let region_provider = RegionProviderChain::default_provider();
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .profile_name(aws_profile)
        .load()
        .await;

    Client::new(&config)
}
