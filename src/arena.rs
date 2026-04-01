use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use crate::event::Event;

pub async fn run_lightcycle_game(p1: String, p2: String, tx: broadcast::Sender<Event>) {
    let mut rx = tx.subscribe();
    let mut board = vec![vec!['.'; 15]; 15];
    let mut p1_pos = (2, 7);
    let mut p2_pos = (12, 7);
    board[p1_pos.1][p1_pos.0] = 'A';
    board[p2_pos.1][p2_pos.0] = 'B';

    let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("LIGHTCYCLES MATCH INITIATED: {} vs {}", p1, p2) });

    let mut current_player = p1.clone();
    let mut other_player = p2.clone();
    let mut p_char = 'A';

    loop {
        let mut board_str = String::new();
        board_str.push_str("  012345678901234\n");
        for (y, row) in board.iter().enumerate() {
            board_str.push_str(&format!("{:x} ", y % 16));
            for cell in row {
                board_str.push(*cell);
            }
            board_str.push('\n');
        }

        let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("Board State:\n{}", board_str) });

        let prompt = format!("{}|\nBOARD STATE:\n{}\n\nYou are player '{}'. Your head is at row {}, col {}. Respond with JSON containing 'N', 'S', 'E', or 'W'.", current_player, board_str, p_char, if current_player == p1 { p1_pos.1 } else { p2_pos.1 }, if current_player == p1 { p1_pos.0 } else { p2_pos.0 });
        let _ = tx.send(Event { sender: "System".to_string(), action: "arena_turn".to_string(), content: prompt });

        let mut move_dir = None;
        let timeout = sleep(Duration::from_secs(20));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Ok(evt) = rx.recv() => {
                    if evt.sender == current_player && evt.action == "plays_move" {
                        move_dir = Some(evt.content.trim().to_uppercase());
                        break;
                    }
                }
                _ = &mut timeout => {
                    let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("{} timed out and derezzed. {} wins!", current_player, other_player) });
                    return;
                }
            }
        }

        let dir = move_dir.unwrap();
        let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("{} moves {}", current_player, dir) });

        let current_pos = if current_player == p1 { p1_pos } else { p2_pos };
        let mut pos = current_pos;
        let mut crashed = false;
        
        match dir.as_str() {
            "N" => if pos.1 > 0 { pos.1 -= 1; } else { crashed = true; },
            "S" => pos.1 += 1,
            "E" => pos.0 += 1,
            "W" => if pos.0 > 0 { pos.0 -= 1; } else { crashed = true; },
            _ => crashed = true,
        }

        if !crashed && (pos.0 >= 15 || pos.1 >= 15) { crashed = true; }
        if !crashed && board[pos.1][pos.0] != '.' { crashed = true; }

        if crashed {
            let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("{} crashed! {} wins the Lightcycles match!", current_player, other_player) });
            return;
        }

        if current_player == p1 {
            board[p1_pos.1][p1_pos.0] = '#';
            p1_pos = pos;
            board[p1_pos.1][p1_pos.0] = 'A';
        } else {
            board[p2_pos.1][p2_pos.0] = '#';
            p2_pos = pos;
            board[p2_pos.1][p2_pos.0] = 'B';
        }

        std::mem::swap(&mut current_player, &mut other_player);
        p_char = if p_char == 'A' { 'B' } else { 'A' };
    }
}