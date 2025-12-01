use krusty::zkb::{ChannelConfig, Filter, Filters, Killmail};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut km = Killmail {
        kill_id: 12345,
        zkb: krusty::zkb::Zkb {
            href: "https://esi.evetech.net/v1/killmails/130678514/145c457c34ce9c9e8d67e942e764d8f439b22271/".to_string(),
        },
        killmail: None,
    };

    km.fetch_data().await?;

    let filters = vec![ChannelConfig {
        channel_ids: vec![1, 3],
        filters: Filters {
            include_npc: false,
            characters: None,
            corps: Some(Filter {
                includes: vec![98190062],
                excludes: vec![],
            }),
            alliances: None,
            regions: None,
            systems: None,
            ships: None,
        },
    }];

    println!("{:?}", km.filter(&filters));

    Ok(())
}
