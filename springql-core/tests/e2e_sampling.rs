// Copyright (c) 2021 TOYOTA MOTOR CORPORATION. Licensed under MIT OR Apache-2.0.

use std::time::Duration;

use pretty_assertions::assert_eq;
use serde_json::json;
use springql_core::error::Result;
use springql_core::low_level_rs::*;
use springql_foreign_service::sink::ForeignSink;
use springql_foreign_service::source::source_input::ForeignSourceInput;
use springql_foreign_service::source::ForeignSource;
use springql_test_logger::setup_test_logger;

fn apply_ddls(ddls: &[String], config: SpringConfig) -> SpringPipeline {
    let pipeline = spring_open(config).unwrap();
    for ddl in ddls {
        spring_command(&pipeline, ddl).unwrap();
    }
    pipeline
}

fn drain_from_sink(sink: &ForeignSink) -> Vec<serde_json::Value> {
    let mut received = Vec::new();
    while let Some(v) = sink.try_receive(Duration::from_secs(1)) {
        received.push(v);
    }
    received
}

#[test]
fn test_e2e_sampling() -> Result<()> {
    setup_test_logger();

    let json_00_1 = json!({
        "ts": "2020-01-01 00:00:00.000000000",
        "ticker": "ORCL",
        "amount": 10,
    });
    let json_00_2 = json!({
        "ts": "2020-01-01 00:00:09.9999999999",
        "ticker": "GOOGL",
        "amount": 30,
    });
    let json_10_1 = json!({
    "ts": "2020-01-01 00:00:10.0000000000",

        "ticker": "IBM",
        "amount": 50,
    });
    let json_20_1 = json!({
    "ts": "2020-01-01 00:00:20.0000000000",

        "ticker": "IBM",
        "amount": 70,
    });

    let source_input = vec![json_00_1, json_00_2, json_10_1, json_20_1];

    let test_source =
        ForeignSource::start(ForeignSourceInput::new_fifo_batch(source_input.clone())).unwrap();
    let test_sink = ForeignSink::start().unwrap();

    let ddls = vec![
        "
        CREATE SOURCE STREAM source_trade (
          ts TIMESTAMP NOT NULL ROWTIME,    
          ticker TEXT NOT NULL,
          amount INTEGER NOT NULL
        );
        "
        .to_string(),
        "
        CREATE SINK STREAM sink_sampled_trade_amount (
          ts TIMESTAMP NOT NULL ROWTIME,    
          amount INTEGER NOT NULL
        );
        "
        .to_string(),
        "
        CREATE PUMP pu_passthrough AS
          INSERT INTO sink_sampled_trade_amount (ts, amount)
          SELECT STREAM
            FLOOR(ts, DURATION_SECS(10)) AS sampled_ts,
            AVG(amount) AS avg_amount
          FROM source_trade
          GROUP BY sampled_ts;
        "
        .to_string(),
        format!(
            "
        CREATE SINK WRITER tcp_sink_trade FOR sink_sampled_trade_amount
          TYPE NET_SERVER OPTIONS (
            PROTOCOL 'TCP',
            REMOTE_HOST '{remote_host}',
            REMOTE_PORT '{remote_port}'
        );
        ",
            remote_host = test_sink.host_ip(),
            remote_port = test_sink.port()
        ),
        format!(
            "
        CREATE SOURCE READER tcp_trade FOR source_trade
          TYPE NET_SERVER OPTIONS (
            PROTOCOL 'TCP',
            REMOTE_HOST '{remote_host}',
            REMOTE_PORT '{remote_port}'
          );
        ",
            remote_host = test_source.host_ip(),
            remote_port = test_source.port()
        ),
    ];

    let _pipeline = apply_ddls(&ddls, spring_config_default());
    let mut sink_received = drain_from_sink(&test_sink);
    sink_received.sort_by_key(|r| {
        let ts = &r["ts"];
        ts.as_str().unwrap().to_string()
    });

    assert_eq!(
        sink_received[0]["ts"].as_str().unwrap(),
        "2020-01-01 00:00:10.000000000"
    );
    assert_eq!(sink_received[0]["amount"].as_i64().unwrap(), 20,);

    assert_eq!(
        sink_received[1]["ts"].as_str().unwrap(),
        "2020-01-01 00:00:20.000000000"
    );
    assert_eq!(sink_received[0]["amount"].as_i64().unwrap(), 50,);

    Ok(())
}
