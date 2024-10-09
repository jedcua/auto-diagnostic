use crate::datasource::ds::DataSource::{CloudwatchLogInsight, CloudwatchMetric, Ec2, Rds};
use crate::lib::config::{AppDescConfig, CloudwatchLogInsightConfig, CloudwatchMetricConfig, Ec2Config, RdsConfig};
use std::cmp::Ordering;
use std::fmt;
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