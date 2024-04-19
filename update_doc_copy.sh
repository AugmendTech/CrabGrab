mkdir -p docs/macos_docs
cargo doc --no-deps --features metal,iosurface,bitmap,screenshot,wgpu 
cp -rf target/doc/** docs/macos_docs
