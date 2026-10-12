use serde::Deserialize;

pub const API_BASE_URL: &str = "https://www.steamgriddb.com/api/v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtworkType {
    Tall,
    Long,
    Hero,
    Logo,
    Icon,
}

impl ArtworkType {
    const TALL_GRID_DIMENSIONS: &[&str] = &["600x900"];
    const LONG_GRID_DIMENSIONS: &[&str] = &["460x215", "920x430"];

    fn path(&self, game_id: u64) -> String {
        match self {
            Self::Tall | Self::Long => format!("/grids/game/{game_id}"),
            Self::Hero => format!("/heroes/game/{game_id}"),
            Self::Logo => format!("/logos/game/{game_id}"),
            Self::Icon => format!("/icons/game/{game_id}"),
        }
    }

    fn dimensions(&self) -> Option<&[&str]> {
        match self {
            Self::Tall => Some(Self::TALL_GRID_DIMENSIONS),
            Self::Long => Some(Self::LONG_GRID_DIMENSIONS),
            Self::Hero | Self::Logo | Self::Icon => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(default)]
    pub data: Option<T>,
    #[serde(default)]
    pub errors: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct GameSearchResult {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ArtworkAsset {
    pub url: String,
    #[serde(default)]
    pub mime: Option<String>,
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

    pub async fn first_artwork(
        &self,
        game_id: u64,
        artwork_type: ArtworkType,
    ) -> Result<Option<ArtworkAsset>, Box<dyn std::error::Error>> {
        let url = format!("{API_BASE_URL}{}", artwork_type.path(game_id));
        let mut request = self.client.get(url).bearer_auth(&self.api_key);

        if let Some(dimensions) = artwork_type.dimensions() {
            request = request.query(&[("dimensions", dimensions.join(","))]);
        }

        let response = request
            .send()
            .await?
            .error_for_status()?
            .json::<ArtworkListResponse>()
            .await?;

        if !response.success {
            let message = response.errors.unwrap_or_default().join("; ");
            return Err(format!("SteamGridDB artwork request failed: {message}").into());
        }

        Ok(response.data.unwrap_or_default().into_iter().next())
    }

    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let bytes = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        Ok(bytes.to_vec())
    }
}
