# CrabGrab 🦀 🖥️ 🦀 

A cross-platform screen-capturing crate for rust

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

Examples can be found at [crabgrab/examples](examples). You can run the examples from a copy of the repository:

`cargo run --example <example_name>`

Note that feature examples will require that feature:

`cargo run --example <example name> --feature <feature name>`