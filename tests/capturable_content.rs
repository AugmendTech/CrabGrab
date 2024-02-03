use crabgrab::capturable_content::*;
use futures::executor::block_on;

#[test]
fn find_capturable_windows() {
    let content_filter = CapturableContentFilter {
        windows: Some(Default::default()),
        screens: false
    };
    assert!(block_on(CapturableContent::new(content_filter)).is_ok());
}
