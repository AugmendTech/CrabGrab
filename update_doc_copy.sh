mkdir -p macos_docs
cargo doc --no-deps --features metal,iosurface
cp -rf target/doc/**  macos_docs
