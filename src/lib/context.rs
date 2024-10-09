use std::error::Error;
use chrono_tz::Tz;
use crate::datasource::ds::DataSource;
use crate::datasource::ds::DataSource::{AppDescription, CloudwatchLogInsight, CloudwatchMetric, Ec2, Rds};
use crate::lib::args;
use crate::lib::args::Args;
use crate::lib::config::Config;

#[derive(Default)]
pub struct AppContext {
    pub profile: String,
    pub start_time: i64,
    pub end_time: i64,
    pub time_zone: Tz,
    pub data_sources: Vec<DataSource>,
    pub open_ai_api_key: Option<String>,
    pub open_ai_model: String,
    pub open_ai_max_token: u32,
    pub print_prompt_data: bool,
    pub dry_run: bool
}

pub fn build_context(args: Args, config: Config) -> Result<AppContext, Box<dyn Error>> {
    let time_zone = match config.general.time_zone {
        Some(tz) => tz.parse().expect("Unknown time zone"),
        None => Tz::UTC
    };

    let (start_time, end_time) = args::build_start_and_end(&args, time_zone)?;

    let mut data_sources: Vec<DataSource> = Vec::new();

    if let Some(configs) = config.app_description {
        for app_desc_config in configs {
            data_sources.push(AppDescription {
                config: app_desc_config
            });
        }
    }

    if let Some(configs) = config.ec2 {
        for ec2_config in configs {
            data_sources.push(Ec2 {
                config: ec2_config
            });
        }
    }

    if let Some(configs) = config.rds {
        for rds_config in configs {
            data_sources.push(Rds {
                config: rds_config
            });
        }
    }

    if let Some(configs) = config.cloudwatch_metric {
        for cloudwatch_config in configs {
            data_sources.push(CloudwatchMetric {
                config: cloudwatch_config
            });
        }
    }

    if let Some(configs) = config.cloudwatch_log_insight {
        for cloudwatch_config in configs {
            data_sources.push(CloudwatchLogInsight {
                config: cloudwatch_config
            });
        }
    }

    data_sources.sort();

    let context = AppContext {
        profile: String::from(&config.general.profile),
        start_time: start_time.as_millis() as i64,
        end_time: end_time.as_millis() as i64,
        time_zone,
        data_sources,
        open_ai_api_key: config.open_ai.api_key,
        open_ai_model: config.open_ai.model,
        open_ai_max_token: config.open_ai.max_token,
        print_prompt_data: args.print_prompt_data,
        dry_run: args.dry_run
    };

    Ok(context)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lib::config::{AppDescConfig, CloudwatchLogInsightConfig, CloudwatchMetricConfig, Ec2Config, GeneralConfig, OpenAiConfig, RdsConfig};
    use std::matches;
    use crate::datasource::ds::DataSource::CloudwatchLogInsight;

    #[test]
    fn build_context_without_errors() {
        let context = build_context(
            Args {
                file: String::from("file.toml"),
                duration: 60,
                start: None,
                end: None,
                print_prompt_data: true,
                dry_run: false,
            },
            Config {
                general: GeneralConfig {
                    profile: "aws-profile".to_string(),
                    time_zone: Some("Asia/Manila".to_string()),
                },
                open_ai: OpenAiConfig {
                    api_key: Some("openai-api-key".to_string()),
                    model: "gpt-4o".to_string(),
                    max_token: 4096,
                },
                app_description: Some(vec![
                    AppDescConfig {
                        order_no: 5,
                        description: "App description".to_string()
                    },
                ]),
                ec2: Some(vec![
                    Ec2Config {
                        order_no: 4,
                        instance_name: "ec2-instance".to_string()
                    }
                ]),
                rds: Some(vec![
                    RdsConfig {
                        order_no: 3,
                        db_identifier: "rds-instance".to_string()
                    }
                ]),
                cloudwatch_metric: Some(vec![
                    CloudwatchMetricConfig {
                        order_no: 2,
                        dimension_name: "dimension-name".to_string(),
                        dimension_value: "dimension-value".to_string(),
                        metric_identifier: "metric-identifier".to_string(),
                        metric_namespace: "metric-namespace".to_string(),
                        metric_name: "metric-name".to_string(),
                        metric_stat: "metric-stat".to_string(),
                    }
                ]),
                cloudwatch_log_insight: Some(vec![
                    CloudwatchLogInsightConfig {
                        order_no: 1,
                        description: "description".to_string(),
                        log_group_name: "log-group-name".to_string(),
                        query: "query".to_string(),
                        result_columns: vec![
                            "col1".to_string(),
                            "col2".to_string()
                        ],
                    }
                ]),
            }
        ).unwrap();

        assert_eq!(context.profile, "aws-profile");
        assert_eq!(context.time_zone, Tz::Asia__Manila);
        assert_eq!(context.open_ai_api_key, Some("openai-api-key".to_string()));
        assert_eq!(context.open_ai_model, "gpt-4o".to_string());
        assert_eq!(context.open_ai_max_token, 4096);
        assert_eq!(context.data_sources.len(), 5);
        assert!(matches!(context.data_sources[0], CloudwatchLogInsight {..}));
        assert!(matches!(context.data_sources[1], CloudwatchMetric{..}));
        assert!(matches!(context.data_sources[2], Rds{..}));
        assert!(matches!(context.data_sources[3], Ec2{..}));
        assert!(matches!(context.data_sources[4], AppDescription{..}));
    }
}
