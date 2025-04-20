use crossterm::{
    cursor, execute, queue,
    style::{self, Stylize},
    terminal::{Clear, ClearType},
};
use siege::MatchData;
use std::io::{stdout, Write};

#[derive(Default, Debug)]
pub struct ClientInfo {
    pub thread_id: Option<u32>,
    pub process_id: Option<u32>,
    pub game_infos: Vec<MatchData>,
}

impl ClientInfo {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_game_info(&mut self, info: MatchData) {
        if !self.game_infos.contains(&info) {
            self.game_infos.push(info);
        }
    }

    pub fn set_thread_id(&mut self, id: u32) {
        self.thread_id = Some(id);
    }

    pub fn set_process_id(&mut self, id: u32) {
        self.process_id = Some(id);
    }

    fn draw_match_data(&self, data: &MatchData) {
        let mut stdout = stdout();
        let headers = vec![
            "Found".to_string(),
            "Name".to_string(),
            "Level".to_string(),
            "K/D".to_string(),
            "Matches Played".to_string(),
            "Privacy Name Enabled".to_string(),
            "Real Name".to_string(),
            "Tracker User".to_string(),
            "Tracker Premium".to_string(),
            "Suspected Cheater".to_string(),
        ];
        let mut rows = vec![headers.clone()];

        for player in &data.players {
            let row = vec![
                player.is_found.to_string(),
                player.privacy_name.clone().unwrap_or(player.name.clone()),
                player.lifetime_stats.level.to_string(),
                format!("{:.2}", player.lifetime_stats.kd),
                player.lifetime_stats.matches_played.to_string(),
                player.privacy_name_enabled.to_string(),
                if player.privacy_name_enabled {
                    player.name.clone()
                } else {
                    "N/A".to_string()
                },
                player.is_tracker_user.to_string(),
                player.is_tracker_premium.to_string(),
                player.is_suspected_cheater.to_string(),
            ];
            rows.push(row);
        }

        let column_widths: Vec<usize> = rows
            .iter()
            .map(|row| row.iter().map(|cell| cell.len()).collect::<Vec<_>>())
            .fold(vec![0; headers.len()], |mut acc, lengths| {
                for (i, &len) in lengths.iter().enumerate() {
                    acc[i] = acc[i].max(len);
                }
                acc
            });

        let separator = "-".repeat(column_widths.iter().sum::<usize>() + 2 * column_widths.len());
        queue!(stdout, cursor::MoveTo(0, 1), style::Print(separator.clone())).expect("Failed to queue row separator");

        let mut x: usize = 0;
        for (i, header) in headers.iter().enumerate() {
            queue!(
                stdout,
                cursor::MoveTo(x as u16, 0),
                crossterm::style::Print(header.clone().bold().underlined().cyan())
            )
            .expect("Failed to queue header cell");
            x += column_widths[i] + 2;
        }

        for (i, player) in data.players.iter().enumerate() {
            let i = i + 1;
            let y = i * 2;
            let mut x = 0;
            for (j, cell) in rows[i].iter().enumerate() {
                let cell = cell.clone() + &" ".repeat((column_widths[j] + 2) - cell.len());
                let cell = cell.green();
                // let cell = if i % 2 == 0 {
                //     cell.white().on_black()
                // } else {
                //     cell.black().on_grey()
                // };
                let cell = if player.is_suspected_cheater {
                    cell.red()
                } else if !player.is_found {
                    cell.grey()
                } else {
                    cell
                };
                queue!(
                    stdout,
                    cursor::MoveTo(x as u16, y as u16),
                    style::Print(cell)
                )
                .expect("Failed to queue cell printing");
                x += column_widths[j] + 2;
            }
            queue!(stdout, cursor::MoveTo(0, (y + 1) as u16), style::Print(separator.clone())).expect("Failed to queue row separator");
        }
        stdout.flush().expect("Failed to flush stdout");
    }

    pub fn redraw_console(&self) {
        let mut stdout = stdout();
        execute!(stdout, Clear(ClearType::All)).unwrap();

        if let Some(data) = self.game_infos.last() {
            self.draw_match_data(data);
        } else {
            let message = "No data available".red().to_string();
            let (cols, rows) = crossterm::terminal::size().unwrap();
            let x = (cols / 2).saturating_sub((message.len() / 2) as u16);
            let y = rows / 2;

            execute!(
                stdout,
                cursor::MoveTo(x, y),
                crossterm::style::Print(message)
            )
            .unwrap();
        }
        execute!(stdout, cursor::Hide).unwrap();
    }
}
