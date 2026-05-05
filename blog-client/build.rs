// Генерация только клиентского кода из protobuf (заглушка tonic для BlogService).
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/blog.proto");
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["proto/blog.proto"], &["proto"])?;
    Ok(())
}
