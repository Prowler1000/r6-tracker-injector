use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GameInfo {
    #[serde(rename = "currentSeasonId")]
    pub season_id: u32,
    #[serde(rename = "currentSeasonName")]
    pub season_name: String,
    #[serde(rename = "currentSeasonShortName")]
    pub season_short_name: String,
}