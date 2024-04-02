mkdir -p macos_docs
cargo doc --no-deps --features metal,iosurface,bitmap
cp -rf target/doc/** docs/macos_docs
