mkdir windows_docs -Force
cargo doc --no-deps --features dxgi,dx11
Copy-Item -Path ".\target\doc\*" -Destination "windows_docs" -Recurse -Force
