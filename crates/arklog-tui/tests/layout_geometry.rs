use arklog::app_layout;
use ratatui::layout::Rect;

#[test]
fn hilog_viewport_height_comes_from_the_same_compact_layout_as_rendering() {
    let layout = app_layout(Rect::new(0, 0, 110, 24));

    assert_eq!(layout.controls.height, 3);
    assert_eq!(layout.workspace.height, 21);
    assert_eq!(layout.log_content_height, 19);
}
