use crate::lib::config::CloudwatchLogInsightConfig;
use crate::lib::context::DateTimeRange;
use crate::lib::prompt::PromptData;
use aws_sdk_cloudwatchlogs::operation::get_query_results::GetQueryResultsOutput;
use aws_sdk_cloudwatchlogs::operation::start_query::StartQueryOutput;
use aws_sdk_cloudwatchlogs::types::QueryStatus;
use aws_sdk_cloudwatchlogs::Client;
use csv::Writer;
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;
use QueryStatus::{Cancelled, Complete, Failed, Running, Scheduled, Timeout, UnknownValue};

pub trait CloudwatchLogsClient {
    async fn start_query(&self, log_group_name: &str, query: &str, start_time: i64, end_time: i64) -> Result<StartQueryOutput, Box<dyn Error>>;

    async fn get_query_results(&self, query_id: String) -> Result<GetQueryResultsOutput, Box<dyn Error>>;
}

impl CloudwatchLogsClient for Client {
    async fn start_query(&self, log_group_name: &str, query: &str, start_time: i64, end_time: i64) -> Result<StartQueryOutput, Box<dyn Error>> {
        Ok(self.start_query()
            .log_group_name(log_group_name)
            .query_string(query)
            .start_time(start_time)
            .end_time(end_time)
            .send()
            .await?)
    }

    async fn get_query_results(&self, query_id: String) -> Result<GetQueryResultsOutput, Box<dyn Error>> {
        Ok(self.get_query_results()
            .query_id(query_id)
            .send()
            .await?)
    }
}

pub async fn fetch_data(client: impl CloudwatchLogsClient, config: &CloudwatchLogInsightConfig, range: &DateTimeRange) -> Result<PromptData, Box<dyn Error>> {
    let start_time = range.start_time;
    let end_time = range.end_time;

    let response = client.start_query(
        &config.log_group_name,
        &config.query,
        start_time,
        end_time
    ).await?;

    let query_id = response.query_id().expect("Query Id is missing from response");

    let mut poll_response;

    loop {
        poll_response = client.get_query_results(String::from(query_id)).await?;

        match poll_response.status().unwrap() {
            Complete => break,
            Running | Scheduled => sleep(Duration::from_secs(1)).await,
            Cancelled | Failed | Timeout | UnknownValue | &_ => panic!("Unexpected status: {}", poll_response.status().unwrap()),
        }
    }

    Ok(PromptData {
        description: build_description(config),
        data: extract_to_csv(poll_response, config)?
    })
}

fn build_description(config: &CloudwatchLogInsightConfig) -> Vec<String> {
    vec![
        "Information: [Cloudwatch Log Insights]".to_string(),
        format!("Description: [{}]", &config.description),
        format!("Log Group: [`{}`]", &config.log_group_name),
    ]
}

fn extract_to_csv(output: GetQueryResultsOutput, config: &CloudwatchLogInsightConfig) -> Result<Option<String>, Box<dyn Error>> {
    let mut csv_writer = Writer::from_writer(Vec::new());
    csv_writer.write_record(&config.result_columns)?;
    let mut rows = 0;

    let mut columns_iter = config.result_columns.clone().into_iter().cycle();
    let mut column = columns_iter.next().unwrap();

    for result in output.results() {
        let mut values : Vec<String> = Vec::new();

        // Discard '@ptr' from result
        for result_field in result.iter().filter(|r| r.field().unwrap() != "@ptr") {
            let field = result_field.field().unwrap();

            if column == field {
                values.push(result_field.value().unwrap().parse().unwrap());
                column = columns_iter.next().unwrap();
            } else {
                panic!("Expected column not matched! Expected: {column}, Actual: {field}");
            }
        }

        csv_writer.write_record(values)?;
        rows += 1;
    }

    if rows == 0 {
        return Ok(Some("No applicable data found\n".to_string()))
    }

    let csv = String::from_utf8(csv_writer.into_inner()?)?;
    Ok(Some(csv))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_cloudwatchlogs::types::ResultField;
    use std::cell::RefCell;

    struct MockCloudwatchLogsClient {
        status_queue: RefCell<Vec<QueryStatus>>
    }

    impl MockCloudwatchLogsClient {
        fn new(statuses: Vec<QueryStatus>) -> Self {
            MockCloudwatchLogsClient {
                status_queue: RefCell::new(statuses)
            }
        }
    }

    impl CloudwatchLogsClient for MockCloudwatchLogsClient {
        async fn start_query(&self, _: &str, _: &str, _: i64, _: i64) -> Result<StartQueryOutput, Box<dyn Error>> {
            Ok(StartQueryOutput::builder()
                .query_id("query_id".to_string())
                .build())
        }

        async fn get_query_results(&self, _: String) -> Result<GetQueryResultsOutput, Box<dyn Error>> {
            let mut status_queue = self.status_queue.borrow_mut();
            let query_status = status_queue.remove(0);

            Ok(GetQueryResultsOutput::builder()
                .status(query_status.clone())
                .results(vec![
                    ResultField::builder()
                        .field("@ptr")
                        .value("discarded")
                        .build(),
                    ResultField::builder()
                        .field("column1")
                        .value("row1-column1")
                        .build(),
                    ResultField::builder()
                        .field("column2")
                        .value("row1-column2")
                        .build(),
                ])
                .results(vec![
                    ResultField::builder()
                        .field("@ptr")
                        .value("discarded")
                        .build(),
                    ResultField::builder()
                        .field("column1")
                        .value("row2-column1")
                        .build(),
                    ResultField::builder()
                        .field("column2")
                        .value("row2-column2")
                        .build(),
                ])
                .results(vec![
                    ResultField::builder()
                        .field("@ptr")
                        .value("discarded")
                        .build(),
                    ResultField::builder()
                        .field("column1")
                        .value("row3-column1")
                        .build(),
                    ResultField::builder()
                        .field("column2")
                        .value("row3-column2")
                        .build(),
                ])
                .build())
        }
    }

    #[test]
    fn test_build_description() {
        let config = CloudwatchLogInsightConfig {
            description: "Some description".to_string(),
            log_group_name: "log-group-name".to_string(),
            ..CloudwatchLogInsightConfig::default()
        };

        let description = build_description(&config);

        assert_eq!(description.len(), 3);
        assert_eq!(description[0], "Information: [Cloudwatch Log Insights]".to_string());
        assert_eq!(description[1], "Description: [Some description]".to_string());
        assert_eq!(description[2], "Log Group: [`log-group-name`]".to_string());
    }

    #[test]
    fn test_extract_to_csv_empty_row() {
        let output = GetQueryResultsOutput::builder().build();
        let config = CloudwatchLogInsightConfig {
            result_columns: vec!["column1".to_string(), "column2".to_string()],
            ..CloudwatchLogInsightConfig::default()
        };

        let result = extract_to_csv(output, &config).expect("Should extract to csv");

        assert_eq!(result, Some("No applicable data found\n".to_string()));
    }

    #[tokio::test]
    async fn test_fetch_data() {
        let client = MockCloudwatchLogsClient::new(vec![Scheduled, Running, Complete]);
        let config = CloudwatchLogInsightConfig {
            result_columns: vec!["column1".to_string(), "column2".to_string()],
            ..CloudwatchLogInsightConfig::default()
        };
        let range = DateTimeRange::default();

        let prompt_data = fetch_data(client, &config, &range).await.expect("Should extract to csv");

        let expected = [
            "column1,column2\n",
            "row1-column1,row1-column2\n",
            "row2-column1,row2-column2\n",
            "row3-column1,row3-column2\n",
        ].join("");

        assert_eq!(prompt_data.data, Some(expected));
    }

    #[tokio::test]
    #[should_panic(expected = "Expected column not matched! Expected: columnB, Actual: column2")]
    async fn test_extract_to_csv_mismatch_column() {
        let client = MockCloudwatchLogsClient::new(vec![Complete]);
        let config = CloudwatchLogInsightConfig {
            result_columns: vec!["column1".to_string(), "columnB".to_string()],
            ..CloudwatchLogInsightConfig::default()
        };
        let range = DateTimeRange::default();

        fetch_data(client, &config, &range).await.expect("Should extract to csv");
    }

    #[tokio::test]
    #[should_panic(expected = "Unexpected status: Failed")]
    async fn test_fetch_data_failed() {
        let client = MockCloudwatchLogsClient::new(vec![Failed]);
        let config = CloudwatchLogInsightConfig::default();
        let range = DateTimeRange::default();

        fetch_data(client, &config, &range).await.expect("Should extract to csv");
    }

    #[tokio::test]
    #[should_panic(expected = "Unexpected status: Timeout")]
    async fn test_fetch_data_timeout() {
        let client = MockCloudwatchLogsClient::new(vec![Timeout]);
        let config = CloudwatchLogInsightConfig::default();
        let range = DateTimeRange::default();

        fetch_data(client, &config, &range).await.expect("Should extract to csv");
    }

    #[tokio::test]
    #[should_panic(expected = "Unexpected status: Unknown")]
    async fn test_fetch_data_unknown_value() {
        let client = MockCloudwatchLogsClient::new(vec![UnknownValue]);
        let config = CloudwatchLogInsightConfig::default();
        let range = DateTimeRange::default();

        fetch_data(client, &config, &range).await.expect("Should extract to csv");
    }
}
