use serde::Deserialize;

pub const API_BASE_URL: &str = "https://www.steamgriddb.com/api/v2";

pub mod endpoints {
    pub const SEARCH_AUTOCOMPLETE: &str = "/search/autocomplete/{query}";
    pub const GRIDS_BY_GAME_ID: &str = "/grids/game/{game_id}";
    pub const HEROES_BY_GAME_ID: &str = "/heroes/game/{game_id}";
    pub const LOGOS_BY_GAME_ID: &str = "/logos/game/{game_id}";
    pub const ICONS_BY_GAME_ID: &str = "/icons/game/{game_id}";
}

pub const STATIC_IMAGE_TYPE: &str = "static";
pub const NSFW_FILTER: &str = "false";
pub const HUMOR_FILTER: &str = "false";

pub const TALL_GRID_DIMENSIONS: &[&str] = &["600x900"];
pub const LONG_GRID_DIMENSIONS: &[&str] = &["460x215", "920x430"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtworkType {
    Tall,
    Long,
    Hero,
    Logo,
    Icon,
}

#[derive(Debug, Clone, Copy)]
pub struct ArtworkEndpoint {
    pub artwork_type: ArtworkType,
    pub path_template: &'static str,
    pub dimensions: &'static [&'static str],
}

pub const ARTWORK_ENDPOINTS: &[ArtworkEndpoint] = &[
    ArtworkEndpoint {
        artwork_type: ArtworkType::Tall,
        path_template: endpoints::GRIDS_BY_GAME_ID,
        dimensions: TALL_GRID_DIMENSIONS,
    },
    ArtworkEndpoint {
        artwork_type: ArtworkType::Long,
        path_template: endpoints::GRIDS_BY_GAME_ID,
        dimensions: LONG_GRID_DIMENSIONS,
    },
    ArtworkEndpoint {
        artwork_type: ArtworkType::Hero,
        path_template: endpoints::HEROES_BY_GAME_ID,
        dimensions: &[],
    },
    ArtworkEndpoint {
        artwork_type: ArtworkType::Logo,
        path_template: endpoints::LOGOS_BY_GAME_ID,
        dimensions: &[],
    },
    ArtworkEndpoint {
        artwork_type: ArtworkType::Icon,
        path_template: endpoints::ICONS_BY_GAME_ID,
        dimensions: &[],
    },
];

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(default)]
    pub data: Option<T>,
    #[serde(default)]
    pub errors: Option<Vec<String>>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub total: Option<u32>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct GameSearchResult {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub verified: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ArtworkAsset {
    pub id: u64,
    pub url: String,
    #[serde(default)]
    pub thumb: Option<String>,
    #[serde(default)]
    pub mime: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub score: Option<i64>,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub nsfw: Option<bool>,
    #[serde(default)]
    pub humor: Option<bool>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub lock: Option<bool>,
    #[serde(default)]
    pub epilepsy: Option<bool>,
    #[serde(default)]
    pub upvotes: Option<u64>,
    #[serde(default)]
    pub downvotes: Option<u64>,
    #[serde(default)]
    pub author: Option<ArtworkAuthor>,
}

#[derive(Debug, Deserialize)]
pub struct ArtworkAuthor {
    pub name: String,
    #[serde(default)]
    pub steam64: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
}

pub type GameSearchResponse = ApiResponse<Vec<GameSearchResult>>;
pub type ArtworkListResponse = ApiResponse<Vec<ArtworkAsset>>;

pub struct SteamGridDbClient {
    client: reqwest::Client,
    api_key: String,
}

impl SteamGridDbClient {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let api_key = std::env::var("STEAMGRIDDB_API_KEY").map_err(|_| {
            "STEAMGRIDDB_API_KEY is not set. Set it in the environment before requesting artwork."
        })?;

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
        })
    }

    pub async fn search_first_game(
        &self,
        title: &str,
    ) -> Result<Option<GameSearchResult>, Box<dyn std::error::Error>> {
        let mut url = reqwest::Url::parse(API_BASE_URL)?;
        url.path_segments_mut()
            .expect("SteamGridDB API base URL supports path segments")
            .push("search")
            .push("autocomplete")
            .push(title);

        let response = self
            .client
            .get(url)
            .bearer_auth(&self.api_key)
            .send()
            .await?
            .error_for_status()?
            .json::<GameSearchResponse>()
            .await?;

        if !response.success {
            let message = response.errors.unwrap_or_default().join("; ");
            return Err(format!("SteamGridDB search failed: {message}").into());
        }

        Ok(response.data.unwrap_or_default().into_iter().next())
    }
}
