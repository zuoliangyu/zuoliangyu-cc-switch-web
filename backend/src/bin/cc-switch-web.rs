#[tokio::main]
async fn main() {
    if let Err(error) = cc_switch_lib::run_web_server().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
