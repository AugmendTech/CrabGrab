use crabgrab::capturable_content::*;
use futures::executor::block_on;

#[test]
fn find_capturable_windows() {
    let content_filter = CapturableContentFilter {
        windows: Some(CapturableWindowFilter {
            desktop_windows: true,
            onscreen_only: true,
        }),
        displays: false
    };
    let content_result = block_on(CapturableContent::new(content_filter));
    assert!(content_result.is_ok());
    println!("capturable windows: ");
    let content = content_result.unwrap();
    for window in content.windows() {
        println!(" * {}", window.title());
    }
}
