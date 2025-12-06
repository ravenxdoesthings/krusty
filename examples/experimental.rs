#[tokio::main]
async fn main() {
    let config = krusty::config::Config::load("config.yaml".to_string());
    println!("Loaded config: {:#?}", config.filters);

    println!(
        "Compiled filters: {:#?}",
        config.filters.unwrap().get_compiled_filters()
    );
}
