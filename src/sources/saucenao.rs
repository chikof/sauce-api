use crate::{Error, Sauce, SauceItem, SauceResult};
use async_trait::async_trait;
use reqwest::{header, Client};
use serde::Deserialize;
use std::{borrow::Cow, collections::HashMap};

const BASE_URL: &str = "https://saucenao.com/search.php?url={url}&api_key={api_key}";

/// The [`SauceNao`] source.
/// Requires an API key to function.
#[derive(Debug)]
pub struct SauceNao<'a> {
    api_key: Option<Cow<'a, str>>,
}

#[async_trait]
impl<'a> Sauce for SauceNao<'a> {
    async fn build_url(&self, url: &str) -> Result<String, Error> {
        let api_key = self.get_api_key().clone();

        if api_key.is_none() {
            return Err(Error::GenericStr("API_KEY is None"));
        }

        let fmt = {
            let mut vars = HashMap::new();
            let url = urlencoding::encode(url);
            vars.insert("url".to_string(), url.as_ref());
            let api_key = api_key.unwrap();
            vars.insert("api_key".to_string(), api_key.as_ref());

            strfmt::strfmt(BASE_URL, &vars)?
        };

        return Ok(fmt);
    }

    async fn check_sauce(&self, original_url: &str) -> Result<SauceResult, Error> {
        let url = self.build_url(original_url).await?;
        let url = url + "&db=999&output_type=2&testmode=1&numres=16";
        // Moved these to where we need them

        let cli = Client::new();
        let head = cli.head(original_url).send().await?;

        let content_type = head.headers().get(header::CONTENT_TYPE);
        if let Some(content_type) = content_type {
            let content_type = content_type.to_str()?;
            if !content_type.contains("image") {
                return Err(Error::LinkIsNotImage);
            }
        }

        let resp = cli
            .get(&url)
            .header(header::ACCEPT_ENCODING, "utf-8")
            .send()
            .await?;

        let res = resp.json::<ApiResult>().await?;

        let mut result = SauceResult {
            original_url: original_url.to_string(),
            ..SauceResult::default()
        };

        for x in res.results {
            if let Some(links) = x.data.ext_urls {
                let item = SauceItem {
                    similarity: x.header.similarity.parse::<f32>()?,
                    link: links[0].clone(),
                };
                result.items.push(item);
            }
        }

        Ok(result)
    }
}

impl<'a> SauceNao<'a> {
    /// Creates a new [`SauceNao`] source, with a [`None`] api key.
    #[must_use]
    pub const fn new() -> Self {
        SauceNao { api_key: None }
    }

    /// Sets the api key to a given [String].
    pub fn set_api_key(&mut self, api_key: String) {
        self.api_key = Some(Cow::Owned(api_key));
    }

    // @todo: Figure out a method to implement getting remaining API calls
    // @body: This would make it easier to report to clients of the API what the current limits are.
    // /// Gets the amount of remaining API calls
    // /// # Returns
    // /// It returns a tuple in the format of (short, long)
    // /// Where `short` is the amount remaining in 30 seconds,
    // /// and `long` is the amount remaining in the next 24 hours
    // pub async fn get_remaining(&self) -> Result<(i64, i64), SauceError> {
    //     let url = self.build_url("").await?;
    //
    //     let resp = Client::new().get(&url);
    //
    //     Ok((0, 0))
    // }

    const fn get_api_key(&self) -> &Option<Cow<'a, str>> {
        &self.api_key
    }
}

impl<'a> Default for SauceNao<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize, PartialOrd, PartialEq, Clone, Default)]
struct ApiResult {
    pub results: Vec<ApiResultItem>,
}

#[derive(Debug, Deserialize, PartialOrd, PartialEq, Clone, Default)]
struct ApiResultItem {
    pub header: ApiResultItemHeader,
    pub data: ApiResultItemData,
}

#[derive(Debug, Deserialize, PartialOrd, PartialEq, Clone, Default)]
struct ApiResultItemHeader {
    pub similarity: String,
}

#[derive(Debug, Deserialize, PartialOrd, PartialEq, Clone, Default)]
struct ApiResultItemData {
    pub ext_urls: Option<Vec<String>>,
}
