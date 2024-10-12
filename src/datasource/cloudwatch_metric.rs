use crate::lib::config::CloudwatchMetricConfig;
use crate::lib::context::{DateTimeRange};
use crate::lib::prompt::PromptData;
use aws_sdk_cloudwatch::operation::get_metric_data::GetMetricDataOutput;
use aws_sdk_cloudwatch::types::{Dimension, Metric, MetricDataQuery, MetricStat};
use aws_sdk_cloudwatch::Client;
use aws_smithy_types::DateTime;
use csv::Writer;
use std::error::Error;
use crate::datasource::ec2::{fetch_instance, Ec2Client};

pub trait CloudwatchClient {
    async fn get_metric_data(&self, start_time: DateTime, end_time: DateTime, query: MetricDataQuery) -> Result<GetMetricDataOutput, Box<dyn Error>>;
}

impl CloudwatchClient for Client {
    async fn get_metric_data(&self, start_time: DateTime, end_time: DateTime, query: MetricDataQuery) -> Result<GetMetricDataOutput, Box<dyn Error>> {
        Ok(self.get_metric_data()
            .start_time(start_time)
            .end_time(end_time)
            .metric_data_queries(query)
            .send()
            .await?)
    }
}

pub async fn fetch_data(client: impl CloudwatchClient, ec2_client: impl Ec2Client, config: &CloudwatchMetricConfig, range: &DateTimeRange) -> Result<PromptData, Box<dyn Error>> {
    let metric = Metric::builder()
        .metric_name(&config.metric_name)
        .namespace(&config.metric_namespace)
        .dimensions(build_dimension(ec2_client, config).await?)
        .build();

    let metric_stat = MetricStat::builder()
        .metric(metric)
        .stat(&config.metric_stat)
        .period(60)
        .build();

    let query = MetricDataQuery::builder()
        .id(&config.metric_identifier)
        .metric_stat(metric_stat)
        .build();

    let start_time = DateTime::from_millis(range.start_time);
    let end_time = DateTime::from_millis(range.end_time);

    let response = client.get_metric_data(start_time, end_time, query).await?;

    Ok(PromptData {
        description: build_description(config),
        data: extract_to_csv(range, response)?
    })
}

fn build_description(config: &CloudwatchMetricConfig) -> Vec<String> {
    vec![
        format!("Information: [Cloudwatch {}]", &config.metric_namespace),
        format!("Metric: [`{}`]", &config.metric_name),
        format!("Dimension: [`{}:{}`]", &config.dimension_name, &config.dimension_value)
    ]
}

fn extract_to_csv(range: &DateTimeRange, output: GetMetricDataOutput) -> Result<Option<String>, Box<dyn Error>> {
    let mut csv_writer = Writer::from_writer(Vec::new());
    csv_writer.write_record(["timestamp", "value"])?;
    let mut rows = 0;

    for result in output.metric_data_results() {
        let timestamps = result.timestamps();
        let values = result.values();

        for (timestamp, value) in timestamps.iter().rev().zip(values.iter().rev()) {
            let utc_time = chrono::DateTime::from_timestamp_millis(timestamp.to_millis()?).unwrap();
            let local_time = utc_time.with_timezone(&range.time_zone);

            let t = format!("{local_time}");
            let v = value.clone().to_string();
            csv_writer.write_record(&[t, v])?;
            rows += 1;
        }
    }

    if rows ==  0 {
        return Ok(Some("No applicable data found\n".to_string()))
    }

    let csv = String::from_utf8(csv_writer.into_inner()?)?;
    Ok(Some(csv))
}

async fn build_dimension(ec2_client: impl Ec2Client, config: &CloudwatchMetricConfig) -> Result<Dimension, Box<dyn Error>> {
    let dimension_value;

    // If EC2, fetch convert instance name to instance id first
    if config.metric_namespace == "AWS/EC2" {
        let ec2_instance = fetch_instance(ec2_client, &config.dimension_value).await?;
        dimension_value = ec2_instance.instance_id().unwrap().to_string();
    } else {
        dimension_value = config.dimension_value.clone();
    }

    Ok(Dimension::builder()
        .name(&config.dimension_name)
        .value(dimension_value)
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_cloudwatch::types::MetricDataResult;
    use aws_smithy_types::date_time::Format;
    use chrono_tz::Tz;
    use crate::lib::context::AppContext;

    #[test]
    fn test_build_description() {
        let config = CloudwatchMetricConfig {
            metric_namespace: "AWS/EC2".to_string(),
            metric_name: "CPUUtilization".to_string(),
            dimension_name: "InstanceId".to_string(),
            dimension_value: "ec2-instance-name".to_string(),
            ..CloudwatchMetricConfig::default()
        };

        let description = build_description(&config);

        assert_eq!(description.len(), 3);
        assert_eq!(description[0], "Information: [Cloudwatch AWS/EC2]".to_string());
        assert_eq!(description[1], "Metric: [`CPUUtilization`]".to_string());
        assert_eq!(description[2], "Dimension: [`InstanceId:ec2-instance-name`]".to_string());
    }

    #[test]
    fn test_extract_to_csv_empty_row() {
        let context = AppContext { ..AppContext::default() };
        let output = GetMetricDataOutput::builder().build();

        let result = extract_to_csv(&context.range, output).expect("Should extract to csv");

        assert_eq!(result, Some("No applicable data found\n".to_string()));
    }

    #[test]
    fn test_extract_to_csv() {
        let context = AppContext {
            range: DateTimeRange {
                time_zone: Tz::Asia__Manila,
                ..DateTimeRange::default()
            },
            ..AppContext::default()
        };
        let output = GetMetricDataOutput::builder()
            .metric_data_results(MetricDataResult::builder()
                .timestamps(date_time("2023-10-12T09:30:00Z"))
                .values(1.0)

                .timestamps(date_time("2023-10-12T10:00:00Z"))
                .values(2.0)

                .timestamps(date_time("2023-10-12T10:30:00Z"))
                .values(3.0)

                .timestamps(date_time("2023-10-12T11:00:00Z"))
                .values(4.0)

                .build())
            .build();

        let result = extract_to_csv(&context.range, output).expect("Should extract to csv");

        let expected = [
            "timestamp,value\n",
            "2023-10-12 19:00:00 PST,4\n",
            "2023-10-12 18:30:00 PST,3\n",
            "2023-10-12 18:00:00 PST,2\n",
            "2023-10-12 17:30:00 PST,1\n"
        ].join("");

        assert_eq!(result, Some(expected));
    }

    fn date_time(s: &str) -> DateTime {
        DateTime::from_str(s, Format::DateTime).unwrap()
    }
}