use std::time::Duration;

use crabgrab::prelude::*;
use crabgrab::feature::content_picker::{pick_sharable_content, SharableContentPickerConfig};

use futures::executor::block_on;

fn main() {
    let config = SharableContentPickerConfig {
        display: true,
        window: true,
        excluded_apps: vec![]
    };
    let sharable_content = block_on(pick_sharable_content(config));
    println!("sharable content: {}", sharable_content.is_ok()); 
    sharable_content.unwrap();
}