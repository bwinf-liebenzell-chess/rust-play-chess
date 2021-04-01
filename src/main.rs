use chess::{Board, MoveGen, ChessMove, Piece, Color, BitBoard, Square};
use chess::Piece::Pawn;
use std::ops::BitXor;
use smallvec::{SmallVec, smallvec};
use rayon::prelude::*;
use chess::Color::{White, Black};
use std::io::{BufRead, Write};
use vampirc_uci::{UciMessage, parse_one};
use vampirc_uci::uci::UciMessage::{Id, Info};
use chess::BoardStatus::Checkmate;

const RECURSION_DEPTH: usize = 5;

#[derive(Debug)]
enum RecursiveResult {
	Some(Vec<Box<RecursiveResult>>),
	Last((usize, usize)),
}

impl RecursiveResult {
	pub fn moves_len(&self) -> usize {
		let mut res = 0;
		match self {
			RecursiveResult::Some(d) => {
				for e in d {
					res += e.moves_len();
				}
			}
			RecursiveResult::Last(_) => {
				res += 1;
			}
		}
		res
	}
	pub fn calc_val_list(&self, color: Color) -> Vec<usize> {
		match self {
			RecursiveResult::Some(d) => {
				let mut res = vec![];
				for result in d {
					let mut r = result.calc_val_list(color);
					if r.len() == 1 {
						res.push(r[0]);
					} else if r.len() == 0 {
						res.push(0);
					} else {
						let sum: usize = r.iter().sum();
						let e = sum / r.len();
						res.push(e);
					}
				}
				res
			}
			RecursiveResult::Last(e) => {
				let val = match color {
					Color::White => {
						e.clone().0 - e.clone().1
					},
					Color::Black => {
						e.clone().1 - e.clone().0
					}
				};
				vec![val]
			}
		}
	}
}

fn main() {
	let mut board = Board::default();
	for line in std::io::stdin().lock().lines() {
		let msg: UciMessage = parse_one(&line.unwrap());

		match msg {
			UciMessage::Uci => {
				let res = Id {
					author: None,
					name: Some("Rust-Play-Chess".to_owned())
				};
				println!("{}", res.to_string());
				println!("uciok");
				}
			UciMessage::Debug(_) => {}
			UciMessage::IsReady => {
				println!("isready")
			}
			UciMessage::Register { .. } => {}
			UciMessage::Position { startpos, fen, moves } => {
				if startpos {
					board = Board::default();
				}
				for mv in moves {
					board = board.make_move_new(mv);
				}
			}
			UciMessage::SetOption { .. } => {}
			UciMessage::UciNewGame => {
				println!("isready")
			}
			UciMessage::Stop => {}
			UciMessage::PonderHit => {}
			UciMessage::Quit => {}
			UciMessage::Go { .. } => {
				let res = recursiv_move_gen(&board, RECURSION_DEPTH);
				let val = res.calc_val_list(board.side_to_move());
				let moves = MoveGen::new_legal(&board);
				let moves: Vec<ChessMove> = moves.collect();
				let mv = if moves.iter().any(|e| board.make_move_new(*e)
					.status() == Checkmate) {
					let pos = moves.iter().position(|e| board.make_move_new(*e)
						.status() == Checkmate
					).unwrap();
					moves[pos]
				} else {
					let max = val.iter().max().unwrap();
					let pos = val.iter().position(|e| e == max).unwrap();
					let mv = moves[pos];
					mv
				};

				let str = UciMessage::BestMove { best_move: mv, ponder: None }.to_string();
				println!("{}", str);
			}
			UciMessage::Id { .. } => {}
			UciMessage::UciOk => {}
			UciMessage::ReadyOk => {}
			UciMessage::BestMove { .. } => {}
			UciMessage::CopyProtection(_) => {}
			UciMessage::Registration(_) => {}
			UciMessage::Option(_) => {}
			UciMessage::Info(_) => {}
			UciMessage::Unknown(_, _) => {
				let res = UciMessage::info_string("Help".to_owned());
				println!("{}", res.to_string())
			}
		}
	}
	/*
	let board = Board::default();
	let res = recursiv_move_gen(&board, RECURSION_DEPTH);
	let val = res.calc_val_list(White);
	let moves = MoveGen::new_legal(&board);
	let moves: Vec<ChessMove> = moves.collect();
	let max = val.iter().max().unwrap();
	let pos = val.iter().position(|e| e == max).unwrap();
	let mv = moves[pos];
	println!("Move => {:?}", mv);
	 */
}

fn recursiv_move_gen(board: &Board, n: usize) -> RecursiveResult {
	if n == 0 {
		let board = BoardWrapper(*board);
		let val = board.evaluate();
		return RecursiveResult::Last(val);
	}
	let move_gen = MoveGen::new_legal(board);

	let res: Vec<Box<RecursiveResult>> = if n == RECURSION_DEPTH {
		move_gen.par_bridge().map(|mv| {
			let backup = board.make_move_new(mv);
			let e = recursiv_move_gen(&backup, n - 1);
			Box::new(e)
		}).collect()
	} else {
		move_gen.map(|mv| {
			let backup = board.make_move_new(mv);
			let e = recursiv_move_gen(&backup, n - 1);
			Box::new(e)
		}).collect()
	};
	return RecursiveResult::Some(res);
}

struct BoardWrapper(Board);

impl BoardWrapper {
	fn evaluate(&self) -> (usize, usize) {
		let board = self.0;
		let mut pieces_by_type: SmallVec<[(Piece, Square); 32]> = smallvec![];
		for piece in chess::ALL_PIECES.iter() {
			let mut bb = *board.pieces(*piece);
			loop {
				let p = bb.to_square();
				if p.to_index() >= 64 {
					break
				}
				let modi = BitBoard::from_square(p);
				bb = bb.bitxor(modi);
				pieces_by_type.push((*piece, p));
			}
		}
		let mut white = 0;
		let mut black = 0;
		for piece_and_type in pieces_by_type {
			let piece = piece_and_type.0;
			let val = match piece {
				Pawn => 100,
				Piece::Knight => 225,
				Piece::Bishop => 225,
				Piece::Rook => 500,
				Piece::Queen => 900,
				Piece::King => 0,
			};
			let color = board.color_on(piece_and_type.1).unwrap();
			match color {
				Color::White => { white += val }
				Color::Black => { black += val }
			}
		}
		(white, black)
	}
}