use std::time::Duration;

use async_trait::async_trait;
use reqwest::header;
use visdom::{types::BoxDynElement, Vis};

use crate::{error::Error, make_client};

use super::{Item, Output, Source};

/// The [`Yandex`] source.
///
/// Works with `yandex.com`
#[derive(Debug)]
pub struct Yandex;

#[async_trait]
impl Source for Yandex {
    type State = ();

    async fn check(&self, url: &str) -> Result<Output, Error> {
        let client = make_client();

        // Check whether we're dealing with an image
        let head = client.head(url).send().await?;

        let content_type = head.headers().get(header::CONTENT_TYPE);

        if let Some(content_type) = content_type {
            let content_type = content_type.to_str()?;

            if !content_type.contains("image") {
                return Err(Error::LinkIsNotImage);
            }
        } else {
            return Err(Error::LinkIsNotImage);
        }

        // Build the request

        let req = client
            .get("https://yandex.com/images/search")
            .query(&[("url", url), ("rpt", "imageview")])
            .timeout(Duration::from_secs(10));

        let resp = req.send().await?;
        let text = resp.text().await?;
        let html = Vis::load(text)?;
        let sauce_items = html.find(".CbirSites-Items").children("li");

        let mut items = Vec::new();

        for page in sauce_items.into_iter() {
            let page = Self::harvest_page(&page);

            items.extend(page);
        }

        for item in &mut items {
            if item.link.starts_with("//") {
                item.link = format!("https:{}", item.link);
            }
        }

        Ok(Output {
            original_url: url.to_string(),
            items,
        })
    }

    async fn create(_: Self::State) -> Result<Self, Error> {
        Ok(Self)
    }
}

impl Yandex {
    fn harvest_page(page: &BoxDynElement) -> Option<Item> {
        let dom = Vis::dom(page);

        let link = dom.find(".CbirSites-ItemInfo").first().children("a");
        let url = link.attr("href")?;

        Some(Item {
            link: url.to_string(),
            similarity: 100.0, // Yandex doesn't provide similarity
        })
    }
}
