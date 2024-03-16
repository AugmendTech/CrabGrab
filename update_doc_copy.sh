mkdir -p macos_docs
cargo doc --features metal,iosurface
cp -rf target/doc/**  macos_docs
