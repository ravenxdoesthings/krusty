use krusty::{filters, zkb};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut km = zkb::Killmail {
        kill_id: 12345,
        zkb: krusty::zkb::Zkb {
            href: "https://esi.evetech.net/v1/killmails/130678514/145c457c34ce9c9e8d67e942e764d8f439b22271/".to_string(),
            ..Default::default()
        },
        killmail: None,
    };

    km.fetch_data().await?;

    let mut config = filters::Config {
        filter_sets: vec![
            filters::FilterSet {
                guild_id: 100,
                channel_id: 1,
                filters: vec!["corp:98190062".to_string()],
            },
            filters::FilterSet {
                guild_id: 100,
                channel_id: 3,
                filters: vec!["corp:98190062".to_string()],
            },
        ],
        ..Default::default()
    };

    println!("{:?}", config.filter(&km));

    Ok(())
}
