use crate::lib::config::CloudwatchLogInsightConfig;
use crate::lib::context::AppContext;
use crate::lib::prompt::PromptData;
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_sdk_cloudwatchlogs::operation::get_query_results::GetQueryResultsOutput;
use aws_sdk_cloudwatchlogs::types::QueryStatus;
use aws_sdk_cloudwatchlogs::Client;
use csv::Writer;
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;
use QueryStatus::{Cancelled, Complete, Failed, Running, Scheduled, Timeout, UnknownValue};

pub async fn fetch_data(context: &AppContext, config: &CloudwatchLogInsightConfig) -> Result<PromptData, Box<dyn Error>> {
    let client = init_client(&context.profile).await;

    let start_time = context.start_time;
    let end_time = context.end_time;

    let response = client.start_query()
        .log_group_name(&config.log_group_name)
        .query_string(&config.query)
        .start_time(start_time)
        .end_time(end_time)
        .send()
        .await?;

    let query_id = response.query_id().expect("Query Id is missing from response");

    let mut poll_response;

    loop {
        poll_response = client.get_query_results().query_id(query_id).send().await?;

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

        for result_field in result {
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

async fn init_client(aws_profile: &String) -> Client {
    let region_provider = RegionProviderChain::default_provider();
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .profile_name(aws_profile)
        .load()
        .await;

    Client::new(&config)
}

#[cfg(test)]
mod tests {
    use aws_sdk_cloudwatchlogs::types::ResultField;
    use super::*;

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

    #[test]
    fn test_extract_to_csv() {
        let output = GetQueryResultsOutput::builder()
            .results(vec![
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
                    .field("column1")
                    .value("row3-column1")
                    .build(),
                ResultField::builder()
                    .field("column2")
                    .value("row3-column2")
                    .build(),
            ])
            .build();

        let config = CloudwatchLogInsightConfig {
            result_columns: vec!["column1".to_string(), "column2".to_string()],
            ..CloudwatchLogInsightConfig::default()
        };

        let result = extract_to_csv(output, &config).expect("Should extract to csv");
        let expected = [
            "column1,column2\n",
            "row1-column1,row1-column2\n",
            "row2-column1,row2-column2\n",
            "row3-column1,row3-column2\n",
        ].join("");

        assert_eq!(result, Some(expected));
    }

    #[test]
    #[should_panic(expected = "Expected column not matched! Expected: column1, Actual: column2")]
    fn test_extract_to_csv_mismatch_column() {
        let output = GetQueryResultsOutput::builder()
            .results(vec![
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
                    .field("column2")
                    .value("row2-column1")
                    .build(),
                ResultField::builder()
                    .field("column1")
                    .value("row2-column1")
                    .build(),
            ])
            .results(vec![
                ResultField::builder()
                    .field("column1")
                    .value("row3-column1")
                    .build(),
                ResultField::builder()
                    .field("column2")
                    .value("row3-column2")
                    .build(),
            ])
            .build();

        let config = CloudwatchLogInsightConfig {
            result_columns: vec!["column1".to_string(), "column2".to_string()],
            ..CloudwatchLogInsightConfig::default()
        };

        extract_to_csv(output, &config).expect("Should extract to csv");
    }
}
