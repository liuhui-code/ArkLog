use arklog::SessionLogStore;

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
