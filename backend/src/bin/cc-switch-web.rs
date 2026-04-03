#[tokio::main]
async fn main() {
    let env = env_logger::Env::default().default_filter_or("info");
    let _ = env_logger::Builder::from_env(env)
        .format_timestamp_millis()
        .try_init();

    if let Err(error) = cc_switch_lib::run_web_server().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
