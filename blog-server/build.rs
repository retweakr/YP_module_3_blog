// Генерирует типы Rust и код tonic (сервер + клиент) в `OUT_DIR` из `proto/blog.proto`.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/blog.proto");
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/blog.proto"], &["proto"])?;
    Ok(())
}
