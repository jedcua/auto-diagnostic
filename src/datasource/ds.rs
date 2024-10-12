use crate::datasource::ds::DataSource::{CloudwatchLogInsight, CloudwatchMetric, Ec2, Rds};
use crate::datasource::{app_description, cloudwatch_log_insight, cloudwatch_metric, ec2, rds};
use crate::lib::config::{AppDescConfig, CloudwatchLogInsightConfig, CloudwatchMetricConfig, Ec2Config, RdsConfig};
use crate::lib::context::AppContext;
use crate::lib::prompt::PromptData;
use std::cmp::Ordering;
use std::error::Error;
use std::fmt;
use aws_config::BehaviorVersion;
use aws_config::meta::region::RegionProviderChain;
use DataSource::AppDescription;

#[derive(Debug)]
pub enum DataSource {
    AppDescription { config: AppDescConfig },
    Ec2 { config: Ec2Config },
    Rds { config: RdsConfig },
    CloudwatchMetric { config: CloudwatchMetricConfig },
    CloudwatchLogInsight { config: CloudwatchLogInsightConfig }
}

impl DataSource {
    fn order_no(&self) -> u8 {
        match self {
            AppDescription { config, ..} => config.order_no,
            Ec2 { config, .. } => config.order_no,
            Rds { config, .. } => config.order_no,
            CloudwatchMetric { config, .. } => config.order_no,
            CloudwatchLogInsight { config, .. } => config.order_no,
        }
    }

    pub async fn fetch_data(&self, context: &AppContext) -> Result<PromptData, Box<dyn Error>> {
        let region_provider = RegionProviderChain::default_provider();
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .profile_name(&context.profile)
            .load()
            .await;

        let prompt_data = match self {
            AppDescription { config} => {
                app_description::fetch_data(config)
            },
            Ec2 { config } => {
                let client = aws_sdk_ec2::Client::new(&sdk_config);
                ec2::fetch_data(client, config).await?
            },
            Rds { config } => {
                let client = aws_sdk_rds::Client::new(&sdk_config);
                rds::fetch_data(client, config).await?
            },
            CloudwatchMetric { config } => {
                let client = aws_sdk_cloudwatch::Client::new(&sdk_config);
                let ec2_client = aws_sdk_ec2::Client::new(&sdk_config);
                cloudwatch_metric::fetch_data(client, ec2_client, config, &context.range).await?
            },
            CloudwatchLogInsight { config } => {
                let client = aws_sdk_cloudwatchlogs::Client::new(&sdk_config);
                cloudwatch_log_insight::fetch_data(client, config, &context.range).await?
            }
        };

        Ok(prompt_data)
    }
}

impl fmt::Display for DataSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let display_string = match self {
            AppDescription { .. } => "App description".to_string(),
            Ec2 { .. } => "EC2 instance".to_string(),
            Rds { .. } => "RDS instance".to_string(),
            CloudwatchMetric { .. } => "Cloudwatch metric".to_string(),
            CloudwatchLogInsight { .. } => "Cloudwatch log insight".to_string(),
        };
        write!(f, "{display_string}")
    }
}

impl Ord for DataSource {
    fn cmp(&self, other: &Self) -> Ordering {
        self.order_no().cmp(&other.order_no())
    }
}

impl PartialOrd for DataSource {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for DataSource {
    fn eq(&self, other: &Self) -> bool {
        self.order_no() == other.order_no()
    }
}

impl Eq for DataSource {}