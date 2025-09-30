pub fn init_logs() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_line_number(true)
        .with_file(true)
        .init();
}