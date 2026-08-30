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
    state.scroll_up(3, 5);
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

    state.apply_filter("^keep");
    state
        .append_lines(["drop new", "keep new"])
        .expect("later lines");
    assert_eq!(state.raw_count(), 4);
    assert_eq!(state.visible_count(), 2);
    assert_eq!(
        state.visible_window(10).expect("filtered lines"),
        ["keep old", "keep new"]
    );

    state.apply_filter("(");
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
