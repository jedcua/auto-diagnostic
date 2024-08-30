use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub general: GeneralConfig,
    pub open_ai: OpenAiConfig,
    pub app_description: Option<Vec<AppDescConfig>>,
    pub ec2: Option<Vec<Ec2Config>>,
    pub rds: Option<Vec<RdsConfig>>,
    pub cloudwatch_metric: Option<Vec<CloudwatchMetricConfig>>,
    pub cloudwatch_log_insight: Option<Vec<CloudwatchLogInsightConfig>>,
}

#[derive(Deserialize, Debug)]
pub struct GeneralConfig {
    pub profile: String,
    pub time_zone: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct OpenAiConfig {
    pub api_key: String,
    pub model: String,
    pub max_token: u32
}

#[derive(Deserialize, Debug)]
pub struct AppDescConfig {
    pub order_no: u8,
    pub description: String
}

#[derive(Deserialize, Debug)]
pub struct Ec2Config {
    pub order_no: u8,
    pub instance_name: String
}

#[derive(Deserialize, Debug)]
pub struct RdsConfig {
    pub order_no: u8,
    pub db_identifier: String
}

#[derive(Deserialize, Debug)]
pub struct CloudwatchMetricConfig {
    pub order_no: u8,
    pub dimension_name: String,
    pub dimension_value: String,
    pub metric_identifier: String,
    pub metric_namespace: String,
    pub metric_name: String,
    pub metric_stat: String,
}

#[derive(Deserialize, Debug)]
pub struct CloudwatchLogInsightConfig {
    pub order_no: u8,
    pub description: String,
    pub log_group_name: String,
    pub query: String,
    pub result_columns: Vec<String>
}
