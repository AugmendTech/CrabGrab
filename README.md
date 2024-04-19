# CrabGrab 🦀 🖥️ 🦀 
A cross-platform screen-capturing crate for rust

Capturing video from screens and applications can be very hard, and it's even worse when you want to do it in a cross-platform application. CrabGrab makes it easy to do continuous frame capture that can be used for individual screenshots or for capturing video. It also includes common functionality needed for enumerating screens and applications. You can get from a window to a pixel buffer in just a few lines of code that will work on both Windows and MacOS.

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
- Sound capture (WIP)
- Platform specific extension features
- Screenshot facility

Examples
--------

A fully featured AI assistant built on top of CrabGrab can be seen in the open source [Snippy](https://github.com/AugmendTech/cggui) project.

Small examples showing how to use the CrabGrab crate can be found at [crabgrab/examples](examples). You can run the examples from the repository:

`cargo run --example <example_name>`

Note that feature examples will require that feature:

`cargo run --example <example name> --feature <feature name>`
