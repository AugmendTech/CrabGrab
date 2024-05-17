# CrabGrab 🦀 🖥️ 🦀 
A cross-platform screen-capturing crate for rust

[![Crates.io Version](https://img.shields.io/crates/v/crabgrab)](https://crates.io/crates/crabgrab)
[![docs.rs](https://img.shields.io/docsrs/crabgrab)](https://docs.rs/crabgrab/)
[MacOS Documentation](https://augmendtech.github.io/CrabGrab/macos_docs/crabgrab/index.html)


Capturing video from screens and applications can be very hard, and it's even worse when you want to do it in a cross-platform application. CrabGrab makes it easy to do continuous frame capture that can be used for individual screenshots or for capturing video. It also includes common functionality needed for enumerating screens and applications. You can get from a window to a pixel buffer in just a few lines of code that will work on both Windows and MacOS.

```rust
#[tokio::main]
async fn main() { 
    let token = match CaptureStream::test_access(false) {
        Some(token) => token,
        None => CaptureStream::request_access(false).await.expect("Expected capture access")
    };
    let filter = CapturableContentFilter::NORMAL_WINDOWS;
    let content = CapturableContent::new(filter).await.unwrap();
    let config = CaptureConfig::with_display(content.displays().next().unwrap(), CapturePixelFormat::Bgra8888);

    let mut stream = CaptureStream::new(token, config, |stream_event| {
        // The stream_event here could be a video frame or audio frame
        println!("result: {:?}", stream_event);
    }).unwrap();

    std::thread::sleep(Duration::from_millis(2000));

    stream.stop().unwrap();
}
```

With CrabGrab, you can build things like:

1. An AI assistant that can see your screen. A fully functional AI assistant built on top of CrabGrab can be seen in the open source [Snippy](https://github.com/AugmendTech/snippy) project.

![Snippy, an AI assistant](https://github.com/AugmendTech/CrabGrab/blob/main/docs/snippy_chat_cmd.png?raw=true)

2. A screen recording tool like the [Augmend](https://augmend.com) client.

![Augmend, a video capture tool](https://github.com/AugmendTech/CrabGrab/blob/main/docs/augmend.png?raw=true)

Features:
---------
- Screen and window capture supported
- Compatible with multiple GPU APIs:
    - Wgpu
    - DX11
    - DXGI
    - Metal
    - IOSurface
- Easy frame bitmap generation
- Platform specific extension features
- Screenshot facility
- Sound capture (WIP)

Examples
--------

For a full application example, check out [Snippy](https://github.com/AugmendTech/snippy), an AI assistant built on top of CrabGrab.

Small examples showing how to use the CrabGrab crate can be found at [crabgrab/examples](examples). You can run the examples from the repository:

`cargo run --example <example_name>`

Note that feature examples will require that feature:

`cargo run --example <example name> --feature <feature name>`

MacOS Docs
----------
Unfortunately due to our dependence on metal-rs, building docs for macos doesn't work on docs.rs, since they use linux containers. As a workaround, we host macos documentation in this repository - link above.


Contributions
-------------

All contributions are welcome! We are actively working on this project and are looking to expand the capabilities including sound capture, Linux support, and performance improvements.
