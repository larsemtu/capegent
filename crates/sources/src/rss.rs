use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::{strip_html, truncate_chars, RawItem, Source};

pub struct RssSource {
    id: &'static str,
    url: &'static str,
}

impl RssSource {
    pub fn new(id: &'static str, url: &'static str) -> Self {
        Self { id, url }
    }
}

#[async_trait]
impl Source for RssSource {
    fn id(&self) -> &'static str {
        self.id
    }

    async fn fetch(&self, client: &reqwest::Client) -> Result<Vec<RawItem>> {
        let bytes = client
            .get(self.url)
            .send()
            .await
            .with_context(|| format!("GET {}", self.url))?
            .error_for_status()
            .with_context(|| format!("status fra {}", self.url))?
            .bytes()
            .await?;

        let feed = feed_rs::parser::parse(&bytes[..])
            .with_context(|| format!("parse av feed fra {}", self.url))?;

        let items = feed
            .entries
            .into_iter()
            .filter_map(|entry| {
                let url = entry.links.first()?.href.clone();
                let title = strip_html(&entry.title.map(|t| t.content).unwrap_or_default());
                if title.is_empty() {
                    return None;
                }
                let mut summary =
                    strip_html(&entry.summary.map(|t| t.content).unwrap_or_default());
                truncate_chars(&mut summary, 500);
                Some(RawItem {
                    source: self.id.to_string(),
                    title,
                    summary,
                    url,
                    published: entry.published.or(entry.updated),
                })
            })
            .collect();

        Ok(items)
    }
}
