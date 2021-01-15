use crate::eval::eval;
use crate::position::*;
use crate::r#move::*;
use crate::thread::Thread;
use crate::types::*;

use std::sync::atomic::Ordering;
use std::time::Instant;

pub const MAX_DEPTH: i32 = 100;
pub const CHECKUP_NODES: u64 = 2 << 10;

#[derive(Clone)]
pub struct Limits {
    pub start: Instant,

    pub time: u128,
    pub inc: u128,

    pub movetime: u128,
    pub moves_to_go: u128,

    pub depth: i32,
    pub mate: i32,

    pub is_time_limit: bool,
    pub is_infinite: bool,
}

impl Limits {
    fn elapsed(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    fn should_stop(&self) -> bool {
        if self.is_time_limit {
            let elapsed = self.elapsed();
            return elapsed > self.time / self.moves_to_go + self.inc && elapsed > self.movetime;
        }
        false
    }
}

fn printable_score(score: Score) -> (&'static str, Score) {
    if score.abs() >= MATE_IN_MAX {
        if score > 0 {
            ("mate", (MATE - score) / 2 + 1)
        } else {
            ("mate", -(MATE + score) / 2)
        }
    } else {
        ("cp", score)
    }
}

fn print_thinking(thread: &Thread, depth: i32, score: Score) {
    let elapsed = thread.limits.elapsed();
    let (score_type, score) = printable_score(score);
    let nodes = thread.get_global_nodes();
    let nps = (nodes as f64 * 1000.0 / (elapsed as f64 + 1.0)) as u64;
    println!(
        "info depth {} score {} {} time {} nodes {} nps {}",
        depth, score_type, score, elapsed, nodes, nps
    );
}

pub fn start_search(mut thread: Thread) {
    for d in 0..=thread.limits.depth {
        let pos = thread.root.clone();
        let score = search(&mut thread, pos, d, 0);
        if !thread.abort && thread.id == 0 {
            print_thinking(&thread, d, score);
        }
    }

    if thread.id == 0 {
        println!("bestmove {}", thread.best_move.to_str(&thread.ci));
    }
}

fn search(thread: &mut Thread, pos: Position, depth: i32, height: i32) -> Score {
    thread.inc_nodes();
    let mut best_score = -INFINITE;
    let mut best_move = NO_MOVE;

    if depth == 0 {
        return eval(&pos);
    }

    if thread.get_local_nodes() % CHECKUP_NODES == 0
        && (thread.limits.should_stop() || thread.global_abort.load(Ordering::Relaxed))
    {
        thread.abort = true;
    }
    if thread.abort {
        return best_score;
    }

    let mut move_count = 0;

    for mv in pos.gen_pseudo_legals(&thread.ci) {
        let mut new_pos = pos.clone();
        if !new_pos.make_move(mv, &thread.ci) {
            continue;
        }

        move_count += 1;

        let score = -search(thread, new_pos, depth - 1, height + 1);

        if score > best_score {
            best_score = score;
            best_move = mv;
        }
    }

    if move_count == 0 {
        best_score = if pos.in_check(pos.ctm) {
            -MATE + height
        } else {
            0
        };
    }

    if height == 0 {
        thread.best_move = best_move;
    }

    best_score
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            start: Instant::now(),

            time: 0,
            inc: 0,

            movetime: 0,
            moves_to_go: 30,

            depth: 6,
            mate: 0,

            is_time_limit: false,
            is_infinite: false,
        }
    }
}
