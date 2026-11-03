use arklog::{LogQuery, SessionLogStore};

#[test]
fn queries_inclusive_rfc3339_bounds_with_offsets_and_millisecond_precision() {
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines([
            "1793527200.999 before",
            "1793527201.000 first",
            "1793527201.001 middle",
            "1793527201.002 last",
            "1793527201.003 after",
            "unparsed",
        ])
        .unwrap();
    let query: LogQuery = serde_json::from_value(serde_json::json!({
        "start": "2026-11-01T18:00:01+08:00",
        "end": "2026-11-01T10:00:01.002Z"
    }))
    .unwrap();
    let page = store.query_snapshot().unwrap().query(&query).unwrap();
    assert_eq!(
        page.records.iter().map(|r| r.index).collect::<Vec<_>>(),
        [1, 2, 3]
    );
}

#[test]
fn queries_local_calendar_formats_and_date_only_at_midnight() {
    use chrono::{Local, TimeZone};

    let time = Local
        .with_ymd_and_hms(2026, 11, 3, 10, 20, 30)
        .single()
        .unwrap();
    let millis = time.timestamp_millis();
    let midnight = Local
        .with_ymd_and_hms(2026, 11, 3, 0, 0, 0)
        .single()
        .unwrap()
        .timestamp_millis();
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines([
            format!("{}.{:03} midnight", midnight / 1000, midnight % 1000),
            format!("{}.000 exact", millis / 1000),
            format!("{}.123 fractional", millis / 1000),
            format!("{}.124 after", millis / 1000),
            "unparsed".into(),
        ])
        .unwrap();
    for (start, end, expected) in [
        ("2026-11-03 10:20:30", "2026-11-03 10:20:30.123", vec![1, 2]),
        ("2026/11/03 10:20:30", "2026/11/03 10:20:30.123", vec![1, 2]),
        ("2026-11-03T10:20:30", "2026-11-03T10:20:30.123", vec![1, 2]),
        ("2026-11-03 10:20", "2026/11/03 10:21", vec![1, 2, 3]),
        ("2026-11-03", "2026/11/03", vec![0]),
    ] {
        let query: LogQuery = serde_json::from_value(serde_json::json!({
            "start": start, "end": end
        }))
        .unwrap();
        let page = store.query_snapshot().unwrap().query(&query).unwrap();
        assert_eq!(
            page.records.iter().map(|r| r.index).collect::<Vec<_>>(),
            expected,
            "{start} .. {end}"
        );
    }
}

#[test]
fn accepts_millisecond_strings_mixed_bounds_and_null_with_exact_epoch_comparisons() {
    use serde_json::json;
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines(["-0.001 before", "0.000 epoch", "0.001 after", "unparsed"])
        .unwrap();
    for (arguments, expected) in [
        (
            json!({"start":"-1", "end":"1970-01-01T00:00:00Z"}),
            vec![0, 1],
        ),
        (
            json!({"start":-1, "end":"1970-01-01T00:00:00Z"}),
            vec![0, 1],
        ),
        (
            json!({"start":"1970-01-01T00:00:00.001Z", "end":null}),
            vec![2],
        ),
        (json!({"start":null, "end":" 1 "}), vec![0, 1, 2]),
        (json!({"start":null, "end":null}), vec![0, 1, 2, 3]),
    ] {
        let query: LogQuery = serde_json::from_value(arguments.clone()).unwrap();
        let page = store.query_snapshot().unwrap().query(&query).unwrap();
        assert_eq!(
            page.records.iter().map(|r| r.index).collect::<Vec<_>>(),
            expected,
            "{arguments}"
        );
    }
}

#[test]
fn rejects_invalid_time_values_and_inverted_mixed_format_ranges() {
    use serde_json::json;
    for value in [
        json!(1.5),
        json!(true),
        json!([]),
        json!({}),
        json!(""),
        json!("11-01 10:00:00.000"),
        json!("2026-02-30 10:00:00"),
        json!("2026-11-03 25:00:00"),
        json!("2026-11-03T10:00:00+25:00"),
        json!("9223372036854775808"),
        json!("2026-11-03 10:00:00 garbage"),
    ] {
        for bound in ["start", "end"] {
            let mut arguments = json!({});
            arguments[bound] = value.clone();
            assert!(
                serde_json::from_value::<LogQuery>(arguments.clone()).is_err(),
                "{arguments}"
            );
        }
    }
    let mut store = SessionLogStore::new().unwrap();
    let query: LogQuery = serde_json::from_value(json!({
        "start":"1970-01-01T00:00:00.001Z", "end":0
    }))
    .unwrap();
    let error = store.query_snapshot().unwrap().query(&query).unwrap_err();
    assert!(error.to_string().contains("start must be <= end"));
}

#[test]
#[cfg(unix)]
fn local_dst_gaps_and_overlaps_require_an_explicit_offset() {
    // Isolate TZ so other tests and Chrono's local timezone cache are unaffected.
    const CHILD: &str = "ARKLOG_TEST_DST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "local_dst_gaps_and_overlaps_require_an_explicit_offset",
            ])
            .env(CHILD, "1")
            .env("TZ", "America/New_York")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    for time in ["2026-03-08 02:30:00", "2026-11-01 01:30:00"] {
        let error =
            serde_json::from_value::<LogQuery>(serde_json::json!({"start":time})).unwrap_err();
        assert!(error.to_string().contains("explicit UTC offset"), "{error}");
    }
    for time in ["2026-03-08T02:30:00-05:00", "2026-11-01T01:30:00-04:00"] {
        assert!(serde_json::from_value::<LogQuery>(serde_json::json!({"start":time})).is_ok());
    }
}

#[test]
fn queries_raw_session_by_inclusive_time_and_regex_without_changing_ui_filter() {
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines([
            "1793527200.000 1 2 I App before",
            "1793527201.000 1 2 E App ERROR first",
            "1793527201.500 1 2 I App unrelated",
            "1793527202.000 1 2 E App error last",
            "1793527202.001 1 2 E App error after",
            "unparsed error",
        ])
        .unwrap();
    store.set_filter("unrelated").unwrap();
    let snapshot = store.query_snapshot().unwrap();
    let query = LogQuery {
        start: Some(1_793_527_201_000),
        end: Some(1_793_527_202_000),
        regex: Some("error (first|last)$".into()),
        ..LogQuery::default()
    };
    let page = snapshot.query(&query).unwrap();
    assert_eq!(
        page.records.iter().map(|r| r.index).collect::<Vec<_>>(),
        [1, 3]
    );
    assert_eq!(page.records[0].text, "1793527201.000 1 2 E App ERROR first");
    assert_eq!(page.raw_count, 6);
    assert_eq!(store.visible_count(), 1);
}

#[test]
fn pages_a_fixed_snapshot_and_rejects_cursors_from_a_cleared_session() {
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines(["match first", "skip", "match second", "match third"])
        .unwrap();
    let first = store
        .query_snapshot()
        .unwrap()
        .query(&LogQuery {
            regex: Some("match".into()),
            limit: 1,
            ..LogQuery::default()
        })
        .unwrap();
    assert_eq!(first.records.len(), 1);
    assert_eq!(first.next_cursor, Some(1));
    store.append_lines(["match later"]).unwrap();
    let next_query = LogQuery {
        regex: Some("match".into()),
        limit: 10,
        cursor: first.next_cursor.unwrap(),
        session_id: Some(first.session_id.clone()),
        until: Some(first.until),
        ..LogQuery::default()
    };
    let frozen = store.query_snapshot().unwrap();
    let next = store.query_snapshot().unwrap().query(&next_query).unwrap();
    assert_eq!(
        next.records.iter().map(|r| r.index).collect::<Vec<_>>(),
        [2, 3]
    );
    assert_eq!(next.next_cursor, None);
    store.clear().unwrap();
    assert!(store.query_snapshot().unwrap().query(&next_query).is_err());
    // A request accepted before Clear can still finish reading the old files.
    assert_eq!(frozen.query(&next_query).unwrap().records.len(), 2);
}

#[test]
fn sparse_queries_yield_bounded_pages_and_keep_a_resumable_cursor() {
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines((0..40).map(|_| "x".repeat(300_000)))
        .unwrap();
    store.append_lines(["needle"]).unwrap();
    let first = store
        .query_snapshot()
        .unwrap()
        .query(&LogQuery {
            regex: Some("needle".into()),
            ..LogQuery::default()
        })
        .unwrap();
    assert!(
        first.records.is_empty(),
        "large sparse scan should yield before reaching the tail"
    );
    let cursor = first.next_cursor.expect("resume scan");
    assert!(cursor > 0 && cursor < 40);
    let next = store
        .query_snapshot()
        .unwrap()
        .query(&LogQuery {
            regex: Some("needle".into()),
            cursor,
            session_id: Some(first.session_id),
            until: Some(first.until),
            ..LogQuery::default()
        })
        .unwrap();
    assert_eq!(next.records[0].text, "needle");
}

#[test]
fn rejects_invalid_query_parameters_without_changing_the_session() {
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines(["1793527200.000 1 2 I App data", "unparsed"])
        .unwrap();
    let queries = [
        LogQuery {
            start: Some(2),
            end: Some(1),
            ..LogQuery::default()
        },
        LogQuery {
            regex: Some("(".into()),
            ..LogQuery::default()
        },
        LogQuery {
            regex: Some("a".repeat(4097)),
            ..LogQuery::default()
        },
        LogQuery {
            limit: 0,
            ..LogQuery::default()
        },
        LogQuery {
            limit: 1001,
            ..LogQuery::default()
        },
        LogQuery {
            cursor: 1,
            ..LogQuery::default()
        },
        LogQuery {
            until: Some(3),
            ..LogQuery::default()
        },
    ];
    for query in queries {
        assert!(
            store.query_snapshot().unwrap().query(&query).is_err(),
            "{query:?}"
        );
    }
    let page = store
        .query_snapshot()
        .unwrap()
        .query(&LogQuery::default())
        .unwrap();
    assert_eq!(page.records.len(), 2);
    assert_eq!(page.records[1].text, "unparsed");
}

#[test]
fn response_budget_does_not_drop_the_first_record_of_the_next_page() {
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines(["a".repeat(200_000), "b".repeat(200_000)])
        .unwrap();
    let page = store
        .query_snapshot()
        .unwrap()
        .query(&LogQuery::default())
        .unwrap();
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.next_cursor, Some(1));
    let next = store
        .query_snapshot()
        .unwrap()
        .query(&LogQuery {
            cursor: 1,
            session_id: Some(page.session_id),
            until: Some(page.until),
            ..LogQuery::default()
        })
        .unwrap();
    assert_eq!(next.records.len(), 1);
    assert_eq!(next.records[0].index, 1);
    assert!(next.next_cursor.is_none());
}

#[test]
fn boolean_query_needs_no_time_and_checks_beyond_the_first_scan_page() {
    use arklog::LogExistsQuery;
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines((0..100_001).map(|_| "ordinary log"))
        .unwrap();
    store.append_lines(["ERROR needle"]).unwrap();
    store.set_filter("hide-all").unwrap();
    assert!(store
        .query_snapshot()
        .unwrap()
        .has_logs(&LogExistsQuery {
            after: None,
            regex: "error needle$".into(),
        })
        .unwrap());
    assert!(!store
        .query_snapshot()
        .unwrap()
        .has_logs(&LogExistsQuery {
            after: None,
            regex: "absent".into(),
        })
        .unwrap());
    assert_eq!(store.visible_count(), 0);
}

#[test]
fn boolean_query_is_strictly_after_unix_milliseconds_and_does_not_return_log_text() {
    use arklog::LogExistsQuery;
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines([
            "1793527200.999 1 2 E App target before",
            "1793527201.000 1 2 E App target at boundary",
            "1793527201.001 1 2 E App unrelated",
            "1793527201.002 1 2 E App TARGET after",
            "unparsed target",
        ])
        .unwrap();
    assert!(store
        .query_snapshot()
        .unwrap()
        .has_logs(&LogExistsQuery {
            after: Some(1_793_527_201_000),
            regex: "target".into(),
        })
        .unwrap());
    assert!(!store
        .query_snapshot()
        .unwrap()
        .has_logs(&LogExistsQuery {
            after: Some(1_793_527_201_002),
            regex: "target".into(),
        })
        .unwrap());
    assert!(store
        .query_snapshot()
        .unwrap()
        .has_logs(&LogExistsQuery {
            after: None,
            regex: "unparsed target".into(),
        })
        .unwrap());
    assert!(store
        .query_snapshot()
        .unwrap()
        .has_logs(&LogExistsQuery {
            after: None,
            regex: "(".into(),
        })
        .is_err());
}

#[test]
fn unix_millisecond_comparisons_are_exact_and_allow_a_single_optional_bound() {
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines([
            "-0.001 1 2 I App before epoch",
            "0.000 1 2 I App epoch",
            "0.001 1 2 I App after",
            "unparsed",
        ])
        .unwrap();
    let page = store
        .query_snapshot()
        .unwrap()
        .query(&LogQuery {
            end: Some(0),
            ..LogQuery::default()
        })
        .unwrap();
    assert_eq!(
        page.records.iter().map(|r| r.index).collect::<Vec<_>>(),
        [0, 1]
    );
    let page = store
        .query_snapshot()
        .unwrap()
        .query(&LogQuery {
            start: Some(1),
            ..LogQuery::default()
        })
        .unwrap();
    assert_eq!(
        page.records.iter().map(|r| r.index).collect::<Vec<_>>(),
        [2]
    );
}

#[test]
fn boolean_match_is_not_limited_by_the_query_text_response_budget() {
    use arklog::LogExistsQuery;
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines([format!(
            "1793527201.001 1 2 E App {} needle",
            "x".repeat(300_000)
        )])
        .unwrap();
    assert!(store
        .query_snapshot()
        .unwrap()
        .has_logs(&LogExistsQuery {
            after: Some(1_793_527_201_000),
            regex: "needle$".into(),
        })
        .unwrap());
}
