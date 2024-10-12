use crate::lib::config::Ec2Config;
use crate::lib::prompt::PromptData;
use aws_sdk_ec2::operation::describe_instances::DescribeInstancesOutput;
use aws_sdk_ec2::types::{Filter, Instance};
use std::error::Error;
use aws_sdk_ec2::Client;

pub trait Ec2Client {
    async fn describe_instances(&self, filter: Filter) -> Result<DescribeInstancesOutput, Box<dyn Error>>;
}

impl Ec2Client for Client {
    async fn describe_instances(&self, filter: Filter) -> Result<DescribeInstancesOutput, Box<dyn Error>> {
        Ok(self.describe_instances()
            .filters(filter)
            .send()
            .await?)
    }
}

pub async fn fetch_instance(client: impl Ec2Client, ec2_instance_name: &String) -> Result<Instance, Box<dyn Error>> {
    let filter = Filter::builder()
        .name("tag:Name")
        .values(ec2_instance_name)
        .build();

    let response = client.describe_instances(filter).await?;

    Ok(response.reservations()
        .first()
        .unwrap()
        .instances()
        .first()
        .unwrap_or_else(|| panic!("Unable to find EC2 instance with name: {ec2_instance_name}"))
        .clone()
    )
}

pub async fn fetch_data(client: impl Ec2Client, config: &Ec2Config) -> Result<PromptData, Box<dyn Error>> {
    let instance = fetch_instance(client, &config.instance_name).await?;

    Ok(PromptData {
        description: build_description(config, instance),
        data: None
    })
}

fn build_description(config: &Ec2Config, instance: Instance) -> Vec<String> {
    let instance_type = instance.instance_type().unwrap().as_str();
    let cpu = instance.cpu_options().unwrap();
    let instance_state = instance.state()
        .unwrap()
        .name()
        .unwrap()
        .to_string();

    vec![
        "Information: [EC2 Instance]".to_string(),
        format!("Instance name: [`{}`]", &config.instance_name),
        format!("Instance type: [`{}`]", instance_type),
        format!("Cpu core count: [{}]", cpu.core_count().unwrap()),
        format!("Cpu threads per core: [{}]", cpu.threads_per_core().unwrap()),
        format!("State: [{instance_state}]"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_ec2::types::builders::InstanceBuilder;
    use aws_sdk_ec2::types::{CpuOptions, InstanceState, InstanceStateName, InstanceType};

    #[test]
    fn test_build_description() {
        let config = Ec2Config {
            order_no: 1,
            instance_name: "ec2-instance".to_string()
        };

        let instance = InstanceBuilder::default()
            .instance_type(InstanceType::T3aMedium)
            .cpu_options(CpuOptions::builder()
                .core_count(1)
                .threads_per_core(2)
                .build()
            )
            .state(InstanceState::builder()
                .name(InstanceStateName::Running)
                .build()
            )
            .build();

        let descriptions = build_description(&config, instance);
        
        assert_eq!(descriptions.len(), 6);
        assert_eq!(descriptions[0], "Information: [EC2 Instance]".to_string());
        assert_eq!(descriptions[1], "Instance name: [`ec2-instance`]".to_string());
        assert_eq!(descriptions[2], "Instance type: [`t3a.medium`]".to_string());
        assert_eq!(descriptions[3], "Cpu core count: [1]".to_string());
        assert_eq!(descriptions[4], "Cpu threads per core: [2]".to_string());
        assert_eq!(descriptions[5], "State: [running]".to_string());
    }
}