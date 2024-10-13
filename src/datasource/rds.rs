use crate::lib::config::RdsConfig;
use crate::lib::prompt::PromptData;
use aws_sdk_rds::operation::describe_db_instances::DescribeDbInstancesOutput;
use aws_sdk_rds::types::DbInstance;
use aws_sdk_rds::Client;
use std::error::Error;

pub trait RdsClient {
    async fn describe_db_instances(&self) -> Result<DescribeDbInstancesOutput, Box<dyn Error>>;
}

impl RdsClient for Client {
    async fn describe_db_instances(&self) -> Result<DescribeDbInstancesOutput, Box<dyn Error>> {
        Ok(self.describe_db_instances()
            .send()
            .await?)
    }
}

pub async fn fetch_data(client: impl RdsClient, config: &RdsConfig) -> Result<PromptData, Box<dyn Error>> {
    let response = client.describe_db_instances().await?;

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

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRdsClient {
        db_instance_identifier: String,
    }

    impl RdsClient for MockRdsClient {
        async fn describe_db_instances(&self) -> Result<DescribeDbInstancesOutput, Box<dyn Error>> {
            Ok(DescribeDbInstancesOutput::builder()
                .db_instances(DbInstance::builder()
                    .db_instance_identifier(&self.db_instance_identifier)
                    .db_instance_class("db.t4g.medium")
                    .engine("postgresql")
                    .engine_version("16.1")
                    .storage_type("some storage")
                    .db_instance_status("running")
                    .multi_az(true)
                    .build())
                .build())
        }
    }

    #[tokio::test]
    async fn test_fetch_data() {
        let client = MockRdsClient {
            db_instance_identifier: "db-identifier-name".to_string()
        };
        let config = RdsConfig {
            order_no: 1,
            db_identifier: "db-identifier-name".to_string(),
        };

        let prompt_data = fetch_data(client, &config).await.expect("Should be able to fetch data");

        assert_eq!(prompt_data.description.len(), 7);
        assert_eq!(prompt_data.description.get(0).unwrap(), "Information: [RDS Instance]");
        assert_eq!(prompt_data.description.get(1).unwrap(), "DB identifier: [`db-identifier-name`]");
        assert_eq!(prompt_data.description.get(2).unwrap(), "Class: [`db.t4g.medium`]");
        assert_eq!(prompt_data.description.get(3).unwrap(), "Engine: [postgresql 16.1]");
        assert_eq!(prompt_data.description.get(4).unwrap(), "Storage type: [some storage]");
        assert_eq!(prompt_data.description.get(5).unwrap(), "Status: [running]");
        assert_eq!(prompt_data.description.get(6).unwrap(), "Multi AZ: [true]");
        assert!(prompt_data.data.is_none());
    }

    #[tokio::test]
    #[should_panic(expected = "Unable to find DB instance with name: db-identifier-name-1")]
    async fn test_fetch_data_not_found() {
        let client = MockRdsClient {
            db_instance_identifier: "db-identifier-name-2".to_string()
        };
        let config = RdsConfig {
            order_no: 1,
            db_identifier: "db-identifier-name-1".to_string(),
        };

        fetch_data(client, &config).await.expect("Should be able to fetch data");
    }
}
