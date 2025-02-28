<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="./docs/logo-light.png">
    <img width="200" height="200" src="./docs/logo-dark.png">
  </picture>
</div>
<hr/>

[![Build](https://github.com/jedcua/auto-diagnostic/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/jedcua/auto-diagnostic/actions/workflows/rust.yml)
[![Crates.io Version](https://img.shields.io/crates/v/auto-diagnostic)](https://crates.io/crates/auto-diagnostic)
[![Coverage](https://codecov.io/github/jedcua/auto-diagnostic/branch/main/graph/badge.svg?token=FG35DKAGJW)](https://codecov.io/github/jedcua/auto-diagnostic)

## Table of Contents

- [Introduction](#introduction)
- [Installation](#installation)
- [Usage](#usage)
- [Datasources](#datasources)

## Introduction
Auto diagnostic is a command line tool that diagnoses an AWS environment using AI.
Under the hood, it does the following:
1. Reading a `toml` configuration file
2. Gathering relevant information from AWS services (EC2, RDS, Cloudwatch, etc.)
3. Building a text prompt from gathered data
4. Asking AI to perform diagnosis from text prompt

## Installation
First, you need to have [Rust](https://www.rust-lang.org/) installed
### From crates.io
```sh
cargo install auto-diagnostic
```

### From Source
```sh
cargo install --git https://github.com/jedcua/auto-diagnostic.git
```

## Usage
```text
Usage: auto-diagnostic [OPTIONS] <FILE>

Arguments:
  <FILE>  Configuration file to use

Options:
      --duration <DURATION>  Duration in seconds, since the current date time [default: 3600]
      --start <START>        Start time
      --end <END>            End time
      --print-prompt-data    Print the raw prompt data
      --dry-run              Dry run mode, don''t generate diagnosis
  -h, --help                 Print help
  -V, --version              Print version
```

Create a `toml` configuration file
```toml
# Required
[general]
profile = 'default'
time_zone = 'Asia/Manila'

# Required
[open_ai]
# Specify your API key here or set OPENAI_API_KEY environment variable
api_key = 'your-openai-api-key'
model = 'gpt-4o'
max_token = 4096

# this is an example of a datasource, define as many as needed for diagnosis
[[ec2]]
order_no = 1
instance_name = 'my-ec2-instance'

[[rds]]
order_no = 2
db_identifier = 'my-rds-instance'

[[cloudwatch_metric]]
order_no = 3
dimension_name = 'DBInstanceIdentifier'
dimension_value = 'my-rds-instance'
metric_identifier = 'rds_cpu_utilization'
metric_namespace = 'AWS/RDS'
metric_name = 'CPUUtilization'
metric_stat = 'Average'
metric_unit = 'percent'

[[cloudwatch_metric]]
order_no = 4
dimension_name = 'DBInstanceIdentifier'
dimension_value = 'my-rds-instance'
metric_identifier = 'rds_byte_balance'
metric_namespace = 'AWS/RDS'
metric_name = 'EBSByteBalance%'
metric_stat = 'Average'

[[cloudwatch_log_insight]]
order_no = 5
description = 'Description related to your query'
log_group_name = '/aws/elasticbeanstalk/var/log/nginx/access.log'
result_columns = ['column1', 'column2', 'column2']
query = '''
    // some LogInsight query
'''
```

Run `auto-diagnostic` with the `toml` file as argument
```shell
$ auto-diagnostic your_file.toml
```

## Datasources
Below are the list of supported datasource you can provide to your `toml` file. 
This can be provided multiple times as needed.

App Description - Provides a user defined description of the application
```toml
[[app_description]]
# The order this data will appear on the text prompt
order_no = 1
# Describe your app that can help with the diagnosis (architecture, programming language, frameworks, etc.)
description = 'This is an awesome app built on ...'
```

EC2 description - Fetches EC2 related information
```toml
[[ec2]]
# The order this data will appear on the text prompt
order_no = 2
# EC2 instance name
instance_name = 'ec2-instance-name'
```

RDS description - Fetches RDS related information
```toml
[[rds]]
# The order this data will appear on the text prompt
order_no = 3
# RDS instance name
db_identifier = 'rds-instance-name'
```

Cloudwatch metric - Fetches a specified metric from Cloudwatch
```toml
[[cloudwatch_metric]]
# The order this data will appear on the text prompt
order_no = 4
# Dimension name to use (e.g. InstanceId, DBInstanceIdentifier)
dimension_name = 'DBInstanceIdentifier'
# Value corresponding to the dimension name (e.g. the name of your RDS instance)
dimension_value = 'your-dimension-value'
# Unique string unsed to identifiy the metric (letters, numbers, and underscore only)
metric_identifier = 'some_unique_metric_identifier'
# Metric namespace (e.g. AWS/EC2, AWS/RDS)
metric_namespace = 'AWS/RDS'
# Metric to fetch (e.g. CPUUtilization, CPUCreditBalance, etc)
metric_name = 'CPUUtilization'
# Metric stat (e.g. Average, Minimum)
metric_stat = 'Average'
# Metric unit, optional
metric_unit = 'percent'
```

Cloudwatch log insight - Executes a query for log insight
```toml
[[cloudwatch_log_insight]]
# The order this data will appear on the text prompt
order_no = 5
# Describe your query, what your query does
description = 'Ranking of the top URL access count from Nginx logs'
# Name of the log group to use
log_group_name = '/aws/elasticbeanstalk/var/log/nginx/access.log'
# The columns here should match what was provided by your query below
result_columns = ['verb', 'url', 'request_count']
# The query to execute
query = '''
    parse @message /(?<verb>(GET|POST|HEAD|PUT|DELETE|OPTIONS)) (?<url>[^\s?]+)/
    | filter verb IN ['GET', 'POST', 'PUT', 'DELETE']
    | stats count() as request_count by verb, url
    | sort request_count desc
    | display verb, url, request_count
    | limit 10
'''
```
