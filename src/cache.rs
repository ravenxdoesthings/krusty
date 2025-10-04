use redis::TypedCommands;

#[derive(Clone)]
pub struct Cache {
    client: redis::Client,
}

impl Cache {
    pub fn new(url: String) -> Self {
        let client = redis::Client::open(url).unwrap();
        Self { client }
    }

    pub fn check(&self, key: &String) -> Result<bool, anyhow::Error> {
        let mut conn = match self.client.get_connection() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = e.to_string(), key, "failed to connect to cache");
                return Err(anyhow::format_err!("failed to connect to cache: {e}"));
            }
        };
        match conn.get(key) {
            Ok(Some(_)) => {
                tracing::debug!(key, "cache hit");
                return Ok(true);
            }
            Ok(None) => {
                tracing::debug!(key, "cache miss");
                return Ok(false);
            }
            Err(e) => {
                tracing::error!(error = e.to_string(), key, "failed to check cache");
                return Err(anyhow::format_err!("failed to retrieve cache item: {e}"));
            }
        }
    }

    pub fn store(
        &self,
        key: &String,
        ttl: Option<std::time::Duration>,
    ) -> Result<(), anyhow::Error> {
        let mut conn = self.client.get_connection()?;
        match ttl {
            None => {
                conn.set(key, 1)?;
            }
            Some(d) => {
                let seconds = d.as_secs();
                conn.set_ex(key, 1, seconds)?;
            }
        }

        Ok(())
    }
}
