```
███╗     █████╗ ██╗   ██╗████████╗ ██████╗
██╔╝    ██╔══██╗██║   ██║╚══██╔══╝██╔═══██╗
██║     ███████║██║   ██║   ██║   ██║   ██║
██║     ██╔══██║██║   ██║   ██║   ██║   ██║
███╗    ██║  ██║╚██████╔╝   ██║   ╚██████╔╝
╚══╝    ╚═╝  ╚═╝ ╚═════╝    ╚═╝    ╚═════╝

██████╗ ██╗ █████╗  ██████╗ ███╗   ██╗ ██████╗ ███████╗████████╗██╗ ██████╗    ███╗
██╔══██╗██║██╔══██╗██╔════╝ ████╗  ██║██╔═══██╗██╔════╝╚══██╔══╝██║██╔════╝    ╚██║
██║  ██║██║███████║██║  ███╗██╔██╗ ██║██║   ██║███████╗   ██║   ██║██║          ██║
██║  ██║██║██╔══██║██║   ██║██║╚██╗██║██║   ██║╚════██║   ██║   ██║██║          ██║
██████╔╝██║██║  ██║╚██████╔╝██║ ╚████║╚██████╔╝███████║   ██║   ██║╚██████╗    ███║
╚═════╝ ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝ ╚══════╝   ╚═╝   ╚═╝ ╚═════╝    ╚══╝
```

## Table of Contents

- [Installation](#installation)
- [Usage](#usage)
- [Datasources](#datasources)


## Installation

### From Source

To install the CLI app from source, you need to have [Rust](https://www.rust-lang.org/) installed. Then run:

```sh
git clone https://github.com/jedcua/auto-diagnostic.git
cd auto-diagnostic
cargo install --path .
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
api_key = 'your-openai-api-key'
model = 'gpt-4o'
max_token = 4096

# this is an example of a datasource, define as many as needed for diagnosis
[[ec2]]
order_no = 1
instance_name = 'your-ec2-instance-name'
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
# Describe your app can help with the diagnosis
description = 'This is an awesome app built on ...'

```
EC2 description - Fetches EC2 related information
```toml
[[ec2]]
# The order this data will appear on the text prompt
order_no = 2
# EC2 instance name
instance_name = '<ec2-instance-name>'
```

RDS description - Fetches RDS related information
```toml
[[rds]]
# The order this data will appear on the text prompt
order_no = 3
# RDS instance name
db_identifier = '<rds-instance-name>'
```

Cloudwatch metric - Fetches a specified metric from Cloudwatch
```toml
[[cloudwatch_metric]]
# The order this data will appear on the text prompt
order_no = 4
# Dimension name to use (e.g. InstanceId, DBInstanceIdentifier)
dimension_name = 'DBInstanceIdentifier'
# Value corresponding to the dimension name (e.g. the name of your RDS instance)
dimension_value = '<your-dimension-value>'
# Unique string unsed to identifiy the metric (letters, numbers, and underscore only)
metric_identifier = 'some_unique_metric_identifier'
# Metric namespace (e.g. AWS/EC2, AWS/RDS)
metric_namespace = 'AWS/RDS'
# Metric to fetch (e.g. CPUUtilization, CPUCreditBalance, etc)
metric_name = 'CPUUtilization'
# Metric stat (e.g. Average, Minimum)
metric_stat = 'Average'
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