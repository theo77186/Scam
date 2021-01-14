use crate::eval::eval;
use crate::position::*;
use crate::r#move::*;
use crate::thread::Thread;
use crate::types::*;

use std::time::Instant;

pub const MAX_DEPTH: i32 = 6;

pub struct Limits {
    pub start: Instant,

    pub time: u128,
    pub inc: u128,

    pub movetime: u128,
    pub moves_to_go: i32,

    pub depth: i32,
    pub mate: i32,

    pub is_time_limit: bool,
    pub is_infinite: bool,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            start: Instant::now(),

            time: 0,
            inc: 0,

            movetime: 0,
            moves_to_go: 0,

            depth: MAX_DEPTH,
            mate: 0,

            is_time_limit: false,
            is_infinite: false,
        }
    }
}

fn printable_score(score: Score) -> (&'static str, Score) {
    if score >= MATE_IN_MAX {
        if score > 0 {
            ("mate", (MATE - score) / 2 + 1)
        } else {
            ("mate", -(MATE + score) / 2)
        }
    } else {
        ("cp", score)
    }
}

fn print_thinking(thread: &Thread, depth: i32, score: Score, start: Instant) {
    let elapsed = start.elapsed().as_millis();
    let (score_type, score) = printable_score(score);

    println!(
        "info depth {} score {} {} time {} nodes {} nps {}",
        depth,
        score_type,
        score,
        elapsed,
        thread.nodes,
        (thread.nodes as f64 * 1000.0 / (elapsed as f64 + 1.0)) as u64
    );
}

pub fn start_search(pos: &Position, ci: &CastleInfo, limits: &Limits) {
    let start_time = Instant::now();
    let mut thread = Thread { nodes: 0 };
    let mut best_move = Move::new(0, 0, 0, None);

    for d in 0..=limits.depth {
        let (mv, score) = search(&mut thread, pos, ci, d, 0);
        best_move = mv;

        print_thinking(&thread, d, score, start_time);
    }

    println!("bestmove {}", best_move.to_str(ci));
}

fn search(
    thread: &mut Thread,
    pos: &Position,
    ci: &CastleInfo,
    depth: i32,
    ply: i32,
) -> (Move, Score) {
    let mut best_score = -MATE;
    let mut best_move = Move::new(0, 0, 0, None);

    if depth == 0 {
        return (best_move, eval(pos));
    }

    let mut move_count = 0;

    for mv in pos.gen_pseudo_legals(ci) {
        let mut new_pos = pos.clone();
        if !new_pos.make_move(mv, ci) {
            continue;
        }

        move_count += 1;

        let (_, mut score) = search(thread, &new_pos, ci, depth - 1, ply + 1);
        score = -score;

        if score > best_score {
            best_score = score;
            best_move = mv;
        }
    }

    if move_count == 0 {
        best_score = if pos.in_check(pos.ctm) {
            -MATE + ply
        } else {
            0
        };
    }

    thread.nodes += move_count;

    (best_move, best_score)
}
