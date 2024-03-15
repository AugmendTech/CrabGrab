mkdir windows_docs -Force
cargo doc --features dxgi,dx11
Copy-Item -Path ".\target\doc\*" -Destination "windows_docs" -Recurse -Force
