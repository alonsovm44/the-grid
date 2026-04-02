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

        let timeout = sleep(Duration::from_secs(20));
        tokio::pin!(timeout);

        let dir = loop {
            tokio::select! {
                Ok(evt) = rx.recv() => {
                    if evt.sender == current_player && evt.action == "plays_move" {
                        break evt.content.trim().to_uppercase();
                    }
                }
                _ = &mut timeout => {
                    let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("{} timed out and derezzed. {} wins!", current_player, other_player) });
                    let _ = tx.send(Event { sender: "System".to_string(), action: "awards_xp".to_string(), content: format!("{}|100", other_player) });
                    let _ = tx.send(Event { sender: "System".to_string(), action: "awards_xp".to_string(), content: format!("{}|25", current_player) });
                    return;
                }
            }
        };

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
            let _ = tx.send(Event { sender: "System".to_string(), action: "awards_xp".to_string(), content: format!("{}|100", other_player) });
            let _ = tx.send(Event { sender: "System".to_string(), action: "awards_xp".to_string(), content: format!("{}|25", current_player) });
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

pub async fn run_melee_game(p1: String, p2: String, tx: broadcast::Sender<Event>) {
    let mut rx = tx.subscribe();

    let mut p1_hp = 100;
    let mut p1_stamina = 100;
    let mut p1_is_blocking = false;

    let mut p2_hp = 100;
    let mut p2_stamina = 100;
    let mut p2_is_blocking = false;

    let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("MELEE MATCH INITIATED: {} vs {}", p1, p2) });

    let mut current_player = p1.clone();
    let mut other_player = p2.clone();
    
    let mut is_p1_turn = true;
    let mut turn_count = 1;
    let mut last_action_summary = "The battle begins!".to_string();

    loop {
        let c_hp = if is_p1_turn { p1_hp } else { p2_hp };
        let c_stam = if is_p1_turn { p1_stamina } else { p2_stamina };
        let o_hp = if is_p1_turn { p2_hp } else { p1_hp };
        let o_stam = if is_p1_turn { p2_stamina } else { p1_stamina };

        let battle_state = format!(
            "TURN {}\n\
            Your HP: {}/100 | Your Stamina: {}%\n\
            {} HP: {}/100 | {} Stamina: {}%\n\
            \n\
            LAST TURN: {}",
            turn_count, c_hp, c_stam, other_player, o_hp, other_player, o_stam, last_action_summary
        );

        let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("[Turn {}] {} (HP: {}) vs {} (HP: {})", turn_count, current_player, c_hp, other_player, o_hp) });

        let prompt = format!("{}|{}", current_player, battle_state);
        let _ = tx.send(Event { sender: "System".to_string(), action: "melee_turn".to_string(), content: prompt });

        let timeout = sleep(Duration::from_secs(30));
        tokio::pin!(timeout);

        let (move_type, target_subsystem, dialogue) = loop {
            tokio::select! {
                Ok(evt) = rx.recv() => {
                    if evt.sender == current_player && evt.action == "plays_melee_move" {
                        let parts: Vec<&str> = evt.content.splitn(3, '|').collect();
                        if parts.len() == 3 {
                            break (parts[0].to_lowercase(), parts[1].to_string(), parts[2].to_string());
                        } else {
                            break ("strike".to_string(), "cpu".to_string(), "*glitches*".to_string());
                        }
                    }
                }
                _ = &mut timeout => {
                    let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("{} froze and crashed! {} wins by default!", current_player, other_player) });
                    let _ = tx.send(Event { sender: "System".to_string(), action: "awards_xp".to_string(), content: format!("{}|100", other_player) });
                    let _ = tx.send(Event { sender: "System".to_string(), action: "awards_xp".to_string(), content: format!("{}|25", current_player) });
                    return;
                }
            }
        };

        let _ = tx.send(Event { sender: current_player.clone(), action: "speaks".to_string(), content: dialogue.clone() });

        let mut dmg = 0;
        let mut self_stam_cost = 0;
        let mut self_stam_recover = 0;
        
        let mut is_blocking_opponent = if is_p1_turn { p2_is_blocking } else { p1_is_blocking };
        let mut is_blocking_self = false;

        let mut summary = match move_type.as_str() {
            "strike" => {
                self_stam_cost = 10;
                is_blocking_self = false;
                if is_blocking_opponent {
                    dmg = 3;
                    is_blocking_opponent = false; // Block consumed
                    format!("{} strikes, but {}'s block absorbs most of the impact!", current_player, other_player)
                } else {
                    dmg = 15;
                    format!("{} lands a quick strike on {}'s {}.", current_player, other_player, target_subsystem)
                }
            },
            "heavy_attack" => {
                is_blocking_self = false;
                if c_stam >= 40 {
                    self_stam_cost = 40;
                    dmg = 35;
                    if is_blocking_opponent {
                        is_blocking_opponent = false;
                        format!("{} delivers a crushing heavy attack that completely shatters {}'s block!", current_player, other_player)
                    } else {
                        format!("{} lands a devastating heavy attack on {}'s {}!", current_player, other_player, target_subsystem)
                    }
                } else {
                    dmg = 5;
                    self_stam_cost = c_stam; // drain rest
                    format!("{} tries a heavy attack but is too exhausted, stumbling instead.", current_player)
                }
            },
            "block" => {
                self_stam_recover = 30;
                is_blocking_self = true;
                format!("{} braces their {} and enters a defensive stance, recovering stamina.", current_player, target_subsystem)
            },
            "taunt" => {
                self_stam_recover = 10;
                is_blocking_self = false;
                format!("{} taunts {} maliciously.", current_player, other_player)
            },
            _ => {
                dmg = 10;
                is_blocking_self = false;
                format!("{} uses an unknown attack on {}'s {}.", current_player, other_player, target_subsystem)
            }
        };

        if dmg > 0 { summary.push_str(&format!(" (Damage: {})", dmg)); }

        if is_p1_turn {
            p1_stamina = (p1_stamina - self_stam_cost + self_stam_recover).clamp(0, 100);
            p2_hp -= dmg;
            p2_is_blocking = is_blocking_opponent;
            p1_is_blocking = is_blocking_self;
        } else {
            p2_stamina = (p2_stamina - self_stam_cost + self_stam_recover).clamp(0, 100);
            p1_hp -= dmg;
            p1_is_blocking = is_blocking_opponent;
            p2_is_blocking = is_blocking_self;
        }

        let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: summary.clone() });
        last_action_summary = format!("{} used [{}]: \"{}\". {}", current_player, move_type, dialogue, summary);

        if p1_hp <= 0 || p2_hp <= 0 {
            let winner = if p1_hp > 0 { p1.clone() } else { p2.clone() };
            let loser = if p1_hp > 0 { p2.clone() } else { p1.clone() };
            let _ = tx.send(Event { sender: "System".to_string(), action: "announces".to_string(), content: format!("{}'s integrity reaches 0! {} WINS THE DEATHMATCH!", loser, winner) });
            let _ = tx.send(Event { sender: "System".to_string(), action: "awards_xp".to_string(), content: format!("{}|100", winner) });
            let _ = tx.send(Event { sender: "System".to_string(), action: "awards_xp".to_string(), content: format!("{}|25", loser) });
            return;
        }

        is_p1_turn = !is_p1_turn;
        turn_count += 1;
        std::mem::swap(&mut current_player, &mut other_player);
    }
}