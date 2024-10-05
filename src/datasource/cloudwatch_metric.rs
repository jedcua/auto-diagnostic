use crate::ec2::fetch_instance;
use crate::lib::config::CloudwatchMetricConfig;
use crate::lib::prompt::PromptData;
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_sdk_cloudwatch::operation::get_metric_data::GetMetricDataOutput;
use aws_sdk_cloudwatch::types::{Dimension, Metric, MetricDataQuery, MetricStat};
use aws_sdk_cloudwatch::Client;
use aws_smithy_types::DateTime;
use csv::Writer;
use std::error::Error;
use crate::lib::context::AppContext;

pub async fn fetch_data(context: &AppContext, config: &CloudwatchMetricConfig) -> Result<PromptData, Box<dyn Error>> {
    let client = init_client(&context.profile).await;

    let metric = Metric::builder()
        .metric_name(&config.metric_name)
        .namespace(&config.metric_namespace)
        .dimensions(build_dimension(&context.profile, config).await?)
        .build();

    let metric_stat = MetricStat::builder()
        .metric(metric)
        .stat(&config.metric_stat)
        .period(60)
        .build();

    let start_time = DateTime::from_millis(context.start_time);
    let end_time = DateTime::from_millis(context.end_time);

    let response = client.get_metric_data()
        .start_time(start_time)
        .end_time(end_time)
        .metric_data_queries(MetricDataQuery::builder()
            .id(&config.metric_identifier)
            .metric_stat(metric_stat)
            .build()
        )
        .send()
        .await?;

    Ok(PromptData {
        description: vec![
            format!("Information: [Cloudwatch {}]", &config.metric_namespace),
            format!("Metric: [`{}`]", &config.metric_name),
            format!("Dimension: [`{}:{}`]", &config.dimension_name, &config.dimension_value)
        ],
        data: extract_to_csv(context, response)?
    })
}

fn extract_to_csv(context: &AppContext, output: GetMetricDataOutput) -> Result<Option<String>, Box<dyn Error>> {
    let mut csv_writer = Writer::from_writer(Vec::new());
    csv_writer.write_record(["timestamp", "value"])?;
    let mut rows = 0;

    for result in output.metric_data_results().iter() {
        let timestamps = result.timestamps();
        let values = result.values();

        for (timestamp, value) in timestamps.iter().rev().zip(values.iter().rev()) {
            let utc_time = chrono::DateTime::from_timestamp_millis(timestamp.to_millis()?).unwrap();
            let local_time = utc_time.with_timezone(&context.time_zone);

            let t = format!("{}", local_time);
            let v = value.clone().to_string();
            csv_writer.write_record(&[t, v])?;
            rows += 1;
        }
    }

    if rows ==  0 {
        return Ok(Some("No applicable data captured\n".to_string()))
    }

    let csv = String::from_utf8(csv_writer.into_inner()?)?;
    Ok(Some(csv))
}

async fn build_dimension(aws_profile: &String, config: &CloudwatchMetricConfig) -> Result<Dimension, Box<dyn Error>> {
    let dimension_value;

    // If EC2, fetch convert instance name to instance id first
    if config.metric_namespace == "AWS/EC2" {
        let ec2_instance = fetch_instance(aws_profile, &config.dimension_value).await?;
        dimension_value = ec2_instance.instance_id().unwrap().to_string();
    } else {
        dimension_value = config.dimension_value.clone();
    }

    Ok(Dimension::builder()
        .name(&config.dimension_name)
        .value(dimension_value)
        .build())
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
