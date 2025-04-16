use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct PlayerInfo {
    #[serde(rename = "playerId")]
    pub id: String,
    #[serde(rename = "playerName")]
    pub name: String,
    #[serde(rename = "playerPrivacyName")]
    pub privacy_name: Option<String>,
    #[serde(rename = "teamId")]
    pub team_id: Option<String>, // Likely None
    #[serde(rename = "partyId")]
    pub party_id: Option<String>, // Likely None
    #[serde(rename = "isFound")]
    pub is_found: bool,
    #[serde(rename = "isPrivacyNameEnabled")]
    pub privacy_name_enabled: bool,
    #[serde(rename = "isDisconnected")]
    pub is_disconnected: bool,
    #[serde(rename = "isSuspectedCheater")]
    pub is_suspected_cheater: bool,
    #[serde(rename = "isOverwolfAppUser")]
    pub is_tracker_user: bool,
    #[serde(rename = "isPremium")]
    pub is_tracker_premium: bool,
    #[serde(rename = "countryCode")]
    pub country_code: Option<String>,
    #[serde(rename = "lifetimeStats")]
    pub lifetime_stats: LifetimeStats,
    #[serde(rename = "lifetimeRankedStats")]
    pub lifetime_ranked_stats: Option<LifetimeRankedStats>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct LifetimeRankedStats {}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct LifetimeStats {
    pub level: u64,
    pub kd: f32,
    pub kills: u64,
    pub deaths: u64,
    #[serde(rename = "killsPerMatch")]
    pub kpm: f32,
    #[serde(rename = "headshotPct")]
    pub headshot_percent: f32,
    #[serde(rename = "matchesWon")]
    pub matches_won: u64,
    #[serde(rename = "matchesLost")]
    pub matches_lost: u64,
    #[serde(rename = "matchesAbandoned")]
    pub matches_abandoned: u64,
    #[serde(rename = "matchesPlayed")]
    pub matches_played: u64,
    #[serde(rename = "matchWinPct")]
    pub win_percent: f32,
    #[serde(rename = "timePlayed")]
    pub time_played: u64,
}

impl Eq for LifetimeStats {}

#[cfg(test)]
mod tests {
    use super::PlayerInfo;
    //use super::*;
    use serde_json::from_str;
    use std::fs;

    #[test]
    fn basic_test() {
        let mut x = 0;
        loop {
            let file_path = format!(r"C:\Users\braed\Documents\GitHub\r6-tracker-injector\target\output\output_{}.json", x);
            match fs::read_to_string(&file_path) {
                Ok(content) => {
                    let data: serde_json::Value = from_str(&content).expect("Failed to parse JSON");
                    let data = data.get("data").unwrap();
                    if let Some(players_info) = data.get("playersInfo").and_then(|v| v.as_array()) {
                        for player in players_info {
                            let player_info: PlayerInfo = serde_json::from_value(player.clone())
                                .expect("Failed to parse PlayerInfo");
                            println!("Loaded PlayerInfo from {}: {:#?}", file_path, player_info);
                        }
                    } else {
                        println!("No playersInfo array found in {}", file_path);
                    }
                    x += 1;
                }
                Err(_) => {
                    println!("No more files to load after {}", file_path);
                    break;
                }
            }
        }
    }
}
