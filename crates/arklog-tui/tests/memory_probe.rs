use arklog::run_memory_probe;

#[test]
fn stress_probe_keeps_session_heap_bounded_for_one_hundred_thousand_lines() {
    let probe = run_memory_probe(100_000).expect("memory probe");

    assert_eq!(probe.raw_count, 100_000);
    assert_eq!(probe.visible_count, 50_000);
    assert_eq!(probe.find_count, 50_000);
    assert!(probe.fault_log_bytes >= 2 * 1024 * 1024);
    assert!(probe.retained_heap_bytes < 2 * 1024 * 1024);
    assert!(probe.rss_bytes > 0);
    assert!(probe.rss_bytes < 50 * 1024 * 1024);
}
