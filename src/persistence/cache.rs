use redis::TypedCommands;

#[derive(Clone)]
pub struct Cache {
    client: redis::Client,
    url: String,
}

impl Cache {
    pub fn build(url: String) -> Result<Self, anyhow::Error> {
        let client = match redis::Client::open(url.clone()) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(url, error = e.to_string(), "failed to create redis client");
                return Err(anyhow::format_err!("failed to create redis client: {e}"));
            }
        };
        Ok(Self {
            client,
            url: url.clone(),
        })
    }

    pub fn check(&self, key: &str) -> Result<bool, anyhow::Error> {
        let mut conn = match self.client.get_connection() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(
                    url = self.url,
                    error = e.to_string(),
                    key,
                    "failed to connect to cache"
                );
                return Err(anyhow::format_err!("failed to connect to cache: {e}"));
            }
        };
        match conn.get(key) {
            Ok(Some(_)) => {
                tracing::debug!(key, "cache hit");
                Ok(true)
            }
            Ok(None) => {
                tracing::debug!(key, "cache miss");
                Ok(false)
            }
            Err(e) => {
                tracing::error!(
                    url = self.url,
                    error = e.to_string(),
                    key,
                    "failed to check cache"
                );
                Err(anyhow::format_err!("failed to retrieve cache item: {e}"))
            }
        }
    }

    pub fn store(&self, key: &str, ttl: Option<std::time::Duration>) -> Result<(), anyhow::Error> {
        let mut conn = match self.client.get_connection() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(
                    url = self.url,
                    error = e.to_string(),
                    key,
                    "failed to connect to cache"
                );
                return Err(anyhow::format_err!("failed to connect to cache: {e}"));
            }
        };
        match ttl {
            None => {
                conn.set(key, 1)?;
            }
            Some(d) => {
                let seconds = if d.as_secs() == 0 && d.subsec_nanos() > 0 {
                    1
                } else {
                    d.as_secs()
                };
                conn.set_ex(key, 1, seconds)?;
            }
        }

        Ok(())
    }
}
