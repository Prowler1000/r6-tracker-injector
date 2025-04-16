use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MatchInfo {
    #[serde(rename = "matchId")]
    id: String,
    #[serde(rename = "playlistId")]
    playlist_id: Option<String>,
    #[serde(rename = "playlistName")]
    playlist_name: Option<String>,
    #[serde(rename = "isPlaylistSupported")]
    is_playlist_supported: bool,
    #[serde(rename = "platformFamilyId")]
    platform_id: String,
    #[serde(rename = "platformFamilyName")]
    platform_name: String,
}