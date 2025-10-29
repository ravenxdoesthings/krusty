#[tokio::main]
async fn main() {
    let config = krusty::config::Config::load("config.yaml".to_string());
    println!("Loaded config: {:#?}", config.experimental);

    println!(
        "Compiled filters: {:#?}",
        config.experimental.unwrap().get_compiled_filters()
    );
}
