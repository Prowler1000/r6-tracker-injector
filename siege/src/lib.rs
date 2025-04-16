mod game;
mod match_info;

pub mod player;

pub use game::GameInfo;
pub use match_info::MatchInfo;
use player::PlayerInfo;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, from_value};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MatchData {
    #[serde(rename = "gameInfo")]
    pub game_metadata: GameInfo,
    #[serde(rename = "matchInfo")]
    pub match_metadata: MatchInfo,
    #[serde(rename = "playersInfo")]
    pub players: Vec<PlayerInfo>,
}

impl MatchData {
    pub fn new(json: impl AsRef<str>) -> Option<Self> {
        if let Ok(data) = from_str::<serde_json::Value>(json.as_ref()) {
            if let Some(data) = data.get("data") {
                from_value(data.clone()).ok()
            } else if data.get("gameInfo").is_some() && data.get("matchInfo").is_some() && data.get("playersInfo").is_some() {
                from_value(data.clone()).ok()
            } else {
                None
            }
        } else {
            None
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
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
                    let game_data: MatchData = serde_json::from_value(data.clone()).unwrap();
                    println!("{:#?}", game_data);
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
