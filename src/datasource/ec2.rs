use crate::lib::config::Ec2Config;
use crate::lib::prompt::PromptData;
use aws_sdk_ec2::operation::describe_instances::DescribeInstancesOutput;
use aws_sdk_ec2::types::{Filter, Instance};
use aws_sdk_ec2::Client;
use std::error::Error;

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

pub async fn fetch_instances(client: impl Ec2Client, ec2_instance_name: &String) -> Result<Vec<Instance>, Box<dyn Error>> {
    let filter = Filter::builder()
        .name("tag:Name")
        .values(ec2_instance_name)
        .build();

    let response = client.describe_instances(filter).await?;

    let mut instances: Vec<Instance> = Vec::new();
    for reservation in response.reservations() {
        for instance in reservation.instances() {
            instances.push(instance.clone());
        }
    }

    assert!(!instances.is_empty(), "Unable to find EC2 instance with name: {ec2_instance_name}");
    Ok(instances)
}

pub async fn fetch_data(client: impl Ec2Client, config: &Ec2Config) -> Result<Vec<PromptData>, Box<dyn Error>> {
    let instances = fetch_instances(client, &config.instance_name).await?;

    let prompt_data_vec = instances.into_iter().map(|instance| PromptData {
        description: build_description(config, instance),
        data: None,
    }).collect();

    Ok(prompt_data_vec)
}

fn build_description(config: &Ec2Config, instance: Instance) -> Vec<String> {
    let instance_id = instance.instance_id().unwrap();
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
        format!("Instance id: [`{}`]", instance_id),
        format!("Instance type: [`{}`]", instance_type),
        format!("Cpu core count: [{}]", cpu.core_count().unwrap()),
        format!("Cpu threads per core: [{}]", cpu.threads_per_core().unwrap()),
        format!("State: [{instance_state}]"),
    ]
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use aws_sdk_ec2::types::{CpuOptions, InstanceState, InstanceStateName, InstanceType, Reservation};

    pub struct MockEc2Client {
        pub instance_id: String
    }

    impl Ec2Client for MockEc2Client {
        async fn describe_instances(&self, _: Filter) -> Result<DescribeInstancesOutput, Box<dyn Error>> {
            Ok(DescribeInstancesOutput::builder()
                .reservations(Reservation::builder()
                    .instances(Instance::builder()
                        .instance_id(&self.instance_id)
                        .instance_type(InstanceType::T3aMedium)
                        .cpu_options(CpuOptions::builder()
                            .core_count(1)
                            .threads_per_core(2)
                            .build())
                        .state(InstanceState::builder()
                            .name(InstanceStateName::Running)
                            .build())
                        .build())
                    .build())
                .build())
        }
    }

    #[tokio::test]
    async fn test_fetch_data() {
        let client = MockEc2Client {
            instance_id: "ec2-instance-id".to_string()
        };
        let config = Ec2Config {
            order_no: 1,
            instance_name: "ec2-instance-name".to_string()
        };

        for prompt_data in fetch_data(client, &config).await.expect("Should be able to fetch data") {
            assert_eq!(prompt_data.description.len(), 7);
            assert_eq!(prompt_data.description[0], "Information: [EC2 Instance]".to_string());
            assert_eq!(prompt_data.description[1], "Instance name: [`ec2-instance-name`]".to_string());
            assert_eq!(prompt_data.description[2], "Instance id: [`ec2-instance-id`]".to_string());
            assert_eq!(prompt_data.description[3], "Instance type: [`t3a.medium`]".to_string());
            assert_eq!(prompt_data.description[4], "Cpu core count: [1]".to_string());
            assert_eq!(prompt_data.description[5], "Cpu threads per core: [2]".to_string());
            assert_eq!(prompt_data.description[6], "State: [running]".to_string());
            assert!(prompt_data.data.is_none());
        }
    }

    struct NoInstanceEc2Client { }

    impl Ec2Client for NoInstanceEc2Client {
        async fn describe_instances(&self, _: Filter) -> Result<DescribeInstancesOutput, Box<dyn Error>> {
            Ok(DescribeInstancesOutput::builder()
                .reservations(Reservation::builder().build())
                .build())
        }
    }

    #[tokio::test]
    #[should_panic(expected = "Unable to find EC2 instance with name: not-found-instance-name")]
    async fn test_fetch_instances_not_found() {
        let client = NoInstanceEc2Client {};
        let instance_name = String::from("not-found-instance-name");

        fetch_instances(client, &instance_name).await.unwrap();
    }
}