use arklog::{ArkLogState, LogTab};

#[test]
fn scrolling_keeps_its_anchor_until_follow_latest_resumes() {
    let mut state = ArkLogState::new().expect("state");
    state
        .append_lines((0..20).map(|index| format!("line {index}")))
        .expect("initial lines");

    assert_eq!(
        state.visible_window(5).expect("latest window"),
        ["line 15", "line 16", "line 17", "line 18", "line 19"]
    );
    state.scroll_up(3, 5).expect("scroll up");
    state.append_lines(["line 20"]).expect("later line");
    assert!(!state.is_following_latest());
    assert_eq!(
        state.visible_window(5).expect("anchored window"),
        ["line 12", "line 13", "line 14", "line 15", "line 16"]
    );

    state.follow_latest();
    assert_eq!(
        state.visible_window(5).expect("resumed window"),
        ["line 16", "line 17", "line 18", "line 19", "line 20"]
    );
}

#[test]
fn applying_regex_reindexes_existing_lines_and_reports_invalid_input() {
    let mut state = ArkLogState::new().expect("state");
    state
        .append_lines(["keep old", "drop old"])
        .expect("initial lines");

    state.apply_filter("^keep").expect("valid filter");
    state
        .append_lines(["drop new", "keep new"])
        .expect("later lines");
    assert_eq!(state.raw_count(), 4);
    assert_eq!(state.visible_count(), 2);
    assert_eq!(
        state.visible_window(10).expect("filtered lines"),
        ["keep old", "keep new"]
    );

    assert!(state.apply_filter("(").is_err());
    assert!(state.filter_error().is_some());
    assert_eq!(state.visible_count(), 0);
    assert_eq!(state.raw_count(), 4);
}

#[test]
fn find_navigation_wraps_and_reveals_matches_without_filtering_lines() {
    let mut state = ArkLogState::new().expect("state");
    state
        .append_lines(["Needle first", "other", "needle second"])
        .expect("lines");

    state.set_find("needle").expect("find");
    assert_eq!(state.find_status(), (1, 2));
    assert_eq!(state.current_find_visible_index(), Some(0));
    assert_eq!(state.visible_count(), 3);

    state.next_find().expect("next");
    assert_eq!(state.find_status(), (2, 2));
    assert_eq!(state.current_find_visible_index(), Some(2));
    state.next_find().expect("wrap next");
    assert_eq!(state.current_find_visible_index(), Some(0));
    state.previous_find().expect("wrap previous");
    assert_eq!(state.current_find_visible_index(), Some(2));
    assert!(!state.is_following_latest());
}

#[test]
fn hilog_and_fault_log_share_one_wrapping_tab_state() {
    let mut state = ArkLogState::new().expect("state");
    assert_eq!(state.tab(), LogTab::HiLog);
    state.next_tab();
    assert_eq!(state.tab(), LogTab::FaultLog);
    state.next_tab();
    assert_eq!(state.tab(), LogTab::HiLog);
}

#[test]
fn horizontal_navigation_reaches_the_real_end_and_survives_append_and_resize() {
    let mut state = ArkLogState::new().expect("state");
    state
        .append_lines(["abcdefghijklmnopqrstuvwxyz"])
        .expect("long line");
    state.set_viewport_size(10, 1);

    for _ in 0..10 {
        state.scroll_horizontal_right().expect("scroll right");
    }
    assert_eq!(state.horizontal_offset(), 17);

    state.append_lines(["short"]).expect("later line");
    state.set_viewport_size(12, 1);
    assert_eq!(state.horizontal_offset(), 17);

    state.reset_horizontal();
    assert_eq!(state.horizontal_offset(), 0);
}

#[test]
fn find_reveals_a_match_beyond_the_horizontal_viewport() {
    let mut state = ArkLogState::new().expect("state");
    state
        .append_lines([format!("{}Needle tail", "x".repeat(80))])
        .expect("wide line");
    state.set_viewport_size(12, 1);

    state.set_find("needle").expect("find");

    let offset = state.horizontal_offset();
    assert!(offset > 0);
    assert!(offset <= 80);
    assert!(offset + 12 >= 86);
}

#[test]
fn applying_a_filter_does_not_steal_horizontal_position_for_an_existing_find() {
    let mut state = ArkLogState::new().expect("state");
    state
        .append_lines([format!("{}needle", "x".repeat(80))])
        .expect("wide line");
    state.set_viewport_size(12, 1);
    state.set_find("needle").expect("find");
    assert!(state.horizontal_offset() > 0);
    state.reset_horizontal();

    state.apply_filter(".*").expect("filter");

    assert_eq!(state.horizontal_offset(), 0);
}

#[test]
fn filter_history_records_only_successful_nonempty_applications_and_survives_clear() {
    let mut state = ArkLogState::new().expect("state");
    state.append_lines(["present"]).expect("line");
    assert!(state.apply_filter("(").is_err());
    state.apply_filter("").expect("empty clears filter");
    state
        .apply_filter("^missing$")
        .expect("valid zero-match filter");
    assert_eq!(state.visible_count(), 0);

    state.clear().expect("clear session");

    assert_eq!(
        state.older_filter_query("draft"),
        Some("^missing$".to_string())
    );
    assert_eq!(state.older_filter_query("^missing$"), None);
}

#[test]
fn browsing_filter_history_does_not_change_the_active_filter() {
    let mut state = ArkLogState::new().expect("state");
    state.append_lines(["alpha", "beta"]).expect("lines");
    state.apply_filter("alpha").expect("first filter");
    state.apply_filter("beta").expect("second filter");

    assert_eq!(state.older_filter_query("draft"), Some("beta".to_string()));
    assert_eq!(state.older_filter_query("beta"), Some("alpha".to_string()));
    assert_eq!(state.filter_query(), "beta");
    assert_eq!(state.visible_window(10).expect("active result"), ["beta"]);

    state.cancel_filter_history_navigation();
    assert_eq!(state.filter_query(), "beta");
}

#[test]
fn soft_wrap_keeps_record_identity_and_follows_the_last_wrapped_rows() {
    let mut state = ArkLogState::new().expect("state");
    state
        .append_lines(["abcdefghijklmnopqrstuvwxy"])
        .expect("long record");
    state.set_viewport_size(10, 2);
    state.toggle_soft_wrap();

    let latest = state.display_window().expect("latest wrapped window");
    assert_eq!(state.raw_count(), 1);
    assert_eq!(state.visible_count(), 1);
    assert_eq!(latest.start_record, 0);
    assert_eq!(latest.start_cell, 10);
    assert_eq!(latest.records, ["abcdefghijklmnopqrstuvwxy"]);

    state.scroll_up(1, 2).expect("read earlier wrapped row");
    let earlier = state.display_window().expect("earlier wrapped window");
    assert_eq!(earlier.start_cell, 0);
    assert!(!state.is_following_latest());

    state
        .scroll_down(1, 2)
        .expect("return to latest wrapped rows");
    assert!(state.is_following_latest());
    assert_eq!(state.display_window().expect("latest again").start_cell, 10);
}

#[test]
fn wrapped_content_anchor_survives_resize_append_and_mode_round_trip() {
    let mut state = ArkLogState::new().expect("state");
    state.append_lines(["x".repeat(100)]).expect("long record");
    state.set_viewport_size(10, 3);
    state.toggle_soft_wrap();
    state.scroll_up(2, 3).expect("pause in record");
    let anchor = state.display_window().expect("paused window").start_cell;
    assert!(anchor > 0);

    state.set_viewport_size(14, 3);
    state.append_lines(["later short record"]).expect("append");
    assert_eq!(
        state.display_window().expect("resized window").start_cell,
        anchor
    );

    state.toggle_soft_wrap();
    assert_eq!(
        state.display_window().expect("unwrapped window").start_cell,
        0
    );
    state.toggle_soft_wrap();
    assert_eq!(
        state.display_window().expect("rewrapped window").start_cell,
        anchor
    );
}

#[test]
fn soft_wrap_find_anchors_the_viewport_at_an_offscreen_match() {
    let mut state = ArkLogState::new().expect("state");
    state
        .append_lines([format!("{}Needle tail", "x".repeat(80))])
        .expect("wide line");
    state.set_viewport_size(12, 2);
    state.toggle_soft_wrap();

    state.set_find("needle").expect("find");

    let window = state.display_window().expect("match window");
    assert_eq!(window.start_record, 0);
    assert_eq!(window.start_cell, 80);
    assert_eq!(state.current_find_visible_index(), Some(0));
}

#[test]
fn wrapped_scrolling_moves_by_display_rows_across_record_boundaries() {
    let mut state = ArkLogState::new().expect("state");
    state
        .append_lines(["first".to_string(), "x".repeat(25), "last".to_string()])
        .expect("records");
    state.set_viewport_size(10, 3);
    state.toggle_soft_wrap();

    let latest = state.display_window().expect("latest window");
    assert_eq!((latest.start_record, latest.start_cell), (1, 10));
    assert_eq!(latest.records, ["x".repeat(25), "last".to_string()]);

    state.scroll_up(2, 3).expect("page toward earlier rows");
    let earlier = state.display_window().expect("earlier window");
    assert_eq!((earlier.start_record, earlier.start_cell), (0, 0));

    state.scroll_down(2, 3).expect("page toward latest rows");
    assert!(state.is_following_latest());
    assert_eq!(
        (
            state.display_window().expect("latest again").start_record,
            state.display_window().expect("latest again").start_cell
        ),
        (1, 10)
    );
}
