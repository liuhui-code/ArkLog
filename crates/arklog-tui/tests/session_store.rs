use arklog::SessionLogStore;

#[test]
fn logcat_level_prefix_filters_existing_and_later_lines_by_minimum_severity() {
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines([
            "0 V verbose",
            "1 D debug",
            "2 I message mentions E and F",
            "3 W warning",
            "4 E error",
            "5 F fatal",
            "6 X unknown",
            "unstructured ERROR message",
        ])
        .unwrap();

    store.set_filter("level:WARN").unwrap();
    store
        .append_lines(["7 I later info", "8 E later error"])
        .unwrap();

    assert_eq!(store.raw_count(), 10);
    assert_eq!(
        store.visible_window(0, 20).unwrap(),
        ["3 W warning", "4 E error", "5 F fatal", "8 E later error"]
    );
    store.set_filter("").unwrap();
    assert_eq!(store.visible_count(), 10);
}

#[test]
fn logcat_level_names_are_case_insensitive_and_assert_maps_to_hilog_fatal() {
    let lines = [
        "0 V verbose",
        "1 D debug",
        "2 I info",
        "3 W warn",
        "4 E error",
        "5 F fatal",
    ];
    let mut store = SessionLogStore::new().unwrap();
    store.append_lines(lines).unwrap();
    for (level, start) in [
        ("verbose", 0),
        ("DeBuG", 1),
        ("INFO", 2),
        ("warn", 3),
        ("ERROR", 4),
        ("fatal", 5),
        ("ASSERT", 5),
    ] {
        store.set_filter(&format!("level:{level}")).unwrap();
        assert_eq!(
            store.visible_window(0, 10).unwrap(),
            lines[start..],
            "{level}"
        );
    }
}

#[test]
fn level_and_regex_share_one_rebuild_and_find_only_sees_visible_records() {
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines(["0 I needle", "1 W Needle", "2 E other", "3 F needle"])
        .unwrap();
    store.set_find("needle").unwrap();
    store
        .set_filter("level:WARN (?i)^\\d [WEF] needle$")
        .unwrap();
    assert_eq!(
        store.visible_window(0, 10).unwrap(),
        ["1 W Needle", "3 F needle"]
    );
    assert_eq!(store.find_count(), 2);
    assert_eq!(store.last_rebuild_work().raw_records_scanned, 4);

    store
        .set_filter("level:ERROR (?i)^\\d [WEF] needle$")
        .unwrap();
    assert_eq!(store.visible_window(0, 10).unwrap(), ["3 F needle"]);
    assert_eq!(store.find_count(), 1);
    store
        .set_filter("level:ERROR (?i)^\\d [WEF] needle$")
        .unwrap();
    assert_eq!(store.last_rebuild_work().raw_records_scanned, 0);
    store.append_lines(["4 W needle", "5 E needle"]).unwrap();
    assert_eq!(store.last_rebuild_work().raw_records_scanned, 0);
    assert_eq!(store.find_count(), 2);

    store.clear().unwrap();
    store.append_lines(["6 I needle", "7 E needle"]).unwrap();
    assert_eq!(store.visible_window(0, 10).unwrap(), ["7 E needle"]);
    assert_eq!(store.find_count(), 1);
}

#[test]
fn level_filter_uses_the_same_hilog_layouts_as_level_coloring() {
    let lines = [
        "08-31 20:10:01.001 120 120 W ArkUI/App warning",
        "42 08-31 20:10:01.001 120 120 E ArkUI/App error",
        "1793527201.001 120 120 F ArkUI/App fatal",
        "43 W compact warning",
        "08-31 20:10:01.001 120 120 I ArkUI/App W E F in message",
        "1793527201.001 120 120 X ArkUI/App W in message",
    ];
    let mut store = SessionLogStore::new().unwrap();
    store.append_lines(lines).unwrap();
    store.set_filter("level:WARN").unwrap();
    assert_eq!(store.visible_window(0, 10).unwrap(), lines[..4]);
}

#[test]
fn invalid_level_queries_hide_matches_and_recover_without_losing_raw_records() {
    let mut store = SessionLogStore::new().unwrap();
    store.append_lines(["0 I info", "1 E error"]).unwrap();
    for query in [
        "level:",
        "level:TYPO",
        "level:W+",
        "level:E,F",
        "level:WARN (",
    ] {
        store.set_filter("level:ERROR").unwrap();
        assert!(store.set_filter(query).is_err(), "{query}");
        assert_eq!(store.visible_count(), 0);
        assert_eq!(store.find_count(), 0);
    }
    store.append_lines(["2 F later fatal"]).unwrap();
    assert_eq!(store.visible_count(), 0);
    store.set_filter("level:ERROR").unwrap();
    assert_eq!(
        store.visible_window(0, 10).unwrap(),
        ["1 E error", "2 F later fatal"]
    );
    store.set_filter("").unwrap();
    assert_eq!(store.visible_count(), 3);
    assert_eq!(store.raw_count(), 3);
}

#[test]
fn regex_can_search_literal_level_text_and_keep_its_leading_whitespace() {
    let mut store = SessionLogStore::new().unwrap();
    store
        .append_lines([
            "level:WARN message",
            "  level:WARN indented",
            "0 E level:WARN tag",
        ])
        .unwrap();
    for query in [r"level\:WARN", "(?:level:WARN)"] {
        store.set_filter(query).unwrap();
        assert_eq!(store.visible_count(), 3);
    }
    store.set_filter("  level:WARN").unwrap();
    assert_eq!(
        store.visible_window(0, 10).unwrap(),
        ["  level:WARN indented"]
    );
}

#[test]
fn stores_a_large_session_without_retaining_log_text_on_the_heap() {
    let mut store = SessionLogStore::new().expect("session store");
    store
        .append_lines((0..100_000).map(|index| format!("raw line {index:06}")))
        .expect("append lines");

    assert_eq!(store.raw_count(), 100_000);
    assert_eq!(
        store.visible_window(0, 2).expect("first window"),
        ["raw line 000000", "raw line 000001"]
    );
    assert_eq!(
        store.visible_window(99_998, 10).expect("latest window"),
        ["raw line 099998", "raw line 099999"]
    );
    assert!(store.retained_heap_bytes() >= 4 * 8 * 1024);
    assert!(store.retained_heap_bytes() < 2 * 1024 * 1024);
}

#[test]
fn reindexes_changed_regex_and_indexes_later_lines_incrementally() {
    let mut store = SessionLogStore::new().expect("session store");
    store
        .append_lines(["keep old", "drop old"])
        .expect("initial lines");

    store.set_filter("^keep").expect("valid regex");
    store
        .append_lines(["drop new", "keep new"])
        .expect("later lines");

    assert_eq!(store.raw_count(), 4);
    assert_eq!(store.visible_count(), 2);
    assert_eq!(
        store.visible_window(0, 10).expect("visible lines"),
        ["keep old", "keep new"]
    );
}

#[test]
fn active_regex_append_does_not_rescan_historical_records() {
    let mut store = SessionLogStore::new().expect("session store");
    store
        .append_lines((0..1_000).map(|index| format!("history {index}")))
        .expect("history");
    store.set_filter("^live").expect("filter");
    store
        .set_filter("^live")
        .expect("unchanged filter resets work");

    store
        .append_lines(["drop new", "live new"])
        .expect("incremental append");

    assert_eq!(store.last_rebuild_work().raw_records_scanned, 0);
    assert_eq!(store.visible_count(), 1);
    assert_eq!(
        store.visible_window(0, 10).expect("visible lines"),
        ["live new"]
    );
}

#[test]
fn regex_filter_matches_existing_and_later_lines_without_case_sensitivity() {
    let mut store = SessionLogStore::new().expect("session store");
    store
        .append_lines(["ERROR old", "unrelated"])
        .expect("initial lines");

    store.set_filter("^error").expect("valid regex");
    store
        .append_lines(["Error new", "still unrelated"])
        .expect("later lines");

    assert_eq!(
        store.visible_window(0, 10).expect("visible lines"),
        ["ERROR old", "Error new"]
    );
}

#[test]
fn invalid_or_over_budget_regex_hides_matches_without_discarding_raw_lines() {
    let mut store = SessionLogStore::new().expect("session store");
    store.append_lines(["raw line"]).expect("raw line");

    assert!(store.set_filter("(").is_err());
    store.append_lines(["later raw"]).expect("later raw");
    assert_eq!(store.raw_count(), 2);
    assert_eq!(store.visible_count(), 0);

    assert!(store.set_filter(&"x".repeat(4_097)).is_err());
    assert_eq!(store.raw_count(), 2);
    assert_eq!(store.visible_count(), 0);

    store.set_filter("raw$").expect("replacement regex");
    assert_eq!(
        store.visible_window(0, 10).expect("restored index"),
        ["later raw"]
    );
}

#[test]
fn clear_starts_an_empty_session_and_preserves_the_active_filter() {
    let mut store = SessionLogStore::new().expect("session store");
    store.set_filter("^keep").expect("filter");
    store
        .append_lines(["keep before", "drop before"])
        .expect("old lines");

    store.clear().expect("clear session");
    store
        .append_lines(["drop after", "keep after"])
        .expect("new lines");

    assert_eq!(store.raw_count(), 2);
    assert_eq!(store.visible_count(), 1);
    assert_eq!(
        store.visible_window(0, 10).expect("visible lines"),
        ["keep after"]
    );
}

#[test]
fn find_index_rebuilds_after_filter_changes_and_indexes_later_lines() {
    let mut store = SessionLogStore::new().expect("session store");
    store
        .append_lines(["Needle old", "ignored"])
        .expect("initial lines");

    store.set_find("needle").expect("find query");
    store
        .append_lines(["NEEDLE new", "other"])
        .expect("later lines");
    assert_eq!(store.find_count(), 2);
    assert_eq!(store.find_visible_index(0).expect("first match"), Some(0));
    assert_eq!(store.find_visible_index(1).expect("second match"), Some(2));

    store.set_filter("^needle").expect("filter");
    assert_eq!(store.find_count(), 2);
    assert_eq!(store.find_visible_index(1).expect("rebuilt match"), Some(1));
}

#[test]
fn find_index_uses_original_text_unicode_case_matching() {
    let mut store = SessionLogStore::new().expect("session store");
    store
        .append_lines(["long ſ character", "plain"])
        .expect("lines");

    store.set_find("S").expect("unicode case-insensitive find");

    assert_eq!(store.find_count(), 1);
    assert_eq!(store.find_visible_index(0).expect("match"), Some(0));
}

#[test]
fn query_rebuilds_scan_raw_logs_once_and_find_only_scans_visible_records() {
    let mut store = SessionLogStore::new().expect("session store");
    store
        .append_lines(["keep needle", "drop needle", "keep other", "drop other"])
        .expect("lines");
    store.set_find("needle").expect("initial find");

    store.set_filter("^keep").expect("filter and find rebuild");
    let filter_work = store.last_rebuild_work();
    assert_eq!(filter_work.raw_records_scanned, 4);
    assert_eq!(filter_work.visible_records_scanned, 0);
    assert_eq!(store.find_count(), 1);

    store.set_filter("^keep").expect("unchanged filter");
    let unchanged_work = store.last_rebuild_work();
    assert_eq!(unchanged_work.raw_records_scanned, 0);
    assert_eq!(unchanged_work.visible_records_scanned, 0);

    store.set_find("other").expect("visible-only find rebuild");
    let find_work = store.last_rebuild_work();
    assert_eq!(find_work.raw_records_scanned, 0);
    assert_eq!(find_work.visible_records_scanned, 2);
    assert_eq!(store.find_count(), 1);
}
