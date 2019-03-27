use std::env;
use std::io::{self, Write};
use termion::clear;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::style;

const MINE: char = '*';
const CELL: char = '?';
const FLAG: char = 'F';

const CORNER_TL: char = '┌';
const CORNER_TR: char = '┐';
const CORNER_BL: char = '└';
const CORNER_BR: char = '┘';
const BORDER_HORIZONTAL: char = '─';
const BORDER_VERTICAL: char = '│';

const GAME_OVER: &str = "\r┌──────────────┐\n\r\
                           │  Game Over!  │\n\r\
                           │              │\n\r\
                           │  restart: r  │\n\r\
                           │  quit: q     │\n\r\
                           └──────────────┘";

const HELP_TEXT: &'static str = r"
minesweeper - little terminal minesweeper

flags:
    -w | --width N           ~ set the horizontal count of tiles
    -h | --height N          ~ set the vertical count of tiles
    -d | --difficulty [0, 2] ~ set the difficulty of the game

controls: 
    space: reveal cell
    up/down/left/right: move cursor in direction
    f: flag cell
    q: quit
    r: restart
";

#[derive(Clone, Copy)]
struct Cell {
    mine: bool,
    state: CellState,
}

#[derive(Clone, Copy)]
enum CellState {
    Hidden,
    Revealed,
    Flagged,
}

impl Cell {
    pub fn as_char(&self) -> char {
        match self.state {
            CellState::Hidden => CELL,
            CellState::Revealed if !self.mine => ' ',
            CellState::Flagged => FLAG,
            _ => MINE,
        }
    }
}

struct Game<R, W: Write> {
    input: R,
    output: W,
    cells: Vec<Cell>,
    width: usize,
    height: usize,
    cursor: (usize, usize),
}

impl<R, W: Write> Game<R, W> {
    pub fn new(input: R, output: W, difficulty: u8, width: usize, height: usize) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut cells = Vec::new();

        for _ in 0..width * height {
            cells.push(Cell {
                mine: rng.gen_range(difficulty, 50) < 3,
                state: CellState::Hidden,
            });
        }

        Game {
            input,
            output,
            cells,
            width,
            height,
            cursor: (0, 0),
        }
    }

    fn position_index(&self, x: usize, y: usize) -> usize {
        x + y * self.width
    }

    fn neighbors(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let curr_x = x as isize;
        let curr_y = y as isize;
        let mut neighbors = Vec::new();

        for x in curr_x - 1..=curr_x + 1 {
            for y in curr_y - 1..=curr_y + 1 {
                if x >= 0
                    && y >= 0
                    && (x as usize) < self.width
                    && (y as usize) < self.height
                    && (x != curr_x || y != curr_y)
                {
                    neighbors.push((x as usize, y as usize));
                }
            }
        }

        neighbors
    }

    fn redraw(&mut self) {
        // NOTE: cursor position is based on (1, 1) as top left NOT (0, 0)
        use termion::cursor::Goto;
        write!(self.output, "{}{}{}", Goto(1, 1), style::Reset, CORNER_TL).unwrap();

        // draw top border
        for x in 2..self.width + 2 {
            write!(self.output, "{}{}", Goto(x as u16, 1), BORDER_HORIZONTAL).unwrap();
        }

        write!(self.output, "{}{}", Goto(self.width as u16 + 2, 1), CORNER_TR).unwrap();

        // draw cells
        for y in 0..self.height {
            write!(self.output, "{}{}", Goto(1, y as u16 + 2), BORDER_VERTICAL).unwrap();

            for x in 0..self.width {
                let i = self.position_index(x, y);
                write!(
                    self.output,
                    "{}{}",
                    Goto(x as u16 + 2, y as u16 + 2),
                    self.cells[i].as_char()
                )
                .unwrap();
            }

            write!(
                self.output,
                "{}{}",
                Goto(self.width as u16 + 2, y as u16 + 2),
                BORDER_VERTICAL
            )
            .unwrap();
        }

        // draw bottom border
        write!(
            self.output,
            "{}{}",
            Goto(1, self.height as u16 + 2),
            CORNER_BL
        )
        .unwrap();

        for x in 2..self.width + 2 {
            write!(
                self.output,
                "{}{}",
                Goto(x as u16, self.height as u16 + 2),
                BORDER_HORIZONTAL
            )
            .unwrap();
        }
        write!(
            self.output,
            "{}{}",
            Goto(self.width as u16 + 2, self.height as u16 + 2),
            CORNER_BR
        )
        .unwrap();

        write!(
            self.output,
            "{}{}",
            Goto(self.cursor.0 as u16 + 2, self.cursor.1 as u16 + 2),
            style::Reset
        )
        .unwrap();

        self.output.flush().unwrap();
    }
}

impl<R: Iterator<Item = Result<Key, io::Error>>, W: Write> Game<R, W> {
    fn run(&mut self) {
        write!(self.output, "{}", clear::All).unwrap();

        self.redraw();

        loop {
            let b_key = self
                .input
                .next()
                .expect("input.next() was None!")
                .expect("io error occurred!");

            match b_key {
                Key::Left => self.cursor.0 = (self.cursor.0 as isize - 1).max(0) as usize,
                Key::Right => self.cursor.0 = (self.cursor.0 + 1).min(self.width - 1),
                Key::Up => self.cursor.1 = (self.cursor.1 as isize - 1).max(0) as usize,
                Key::Down => self.cursor.1 = (self.cursor.1 + 1).min(self.height - 1),
                Key::Char(' ') => {
                    let cell = {
                        let i = self.position_index(self.cursor.0, self.cursor.1);
                        self.cells[i]
                    };

                    if let CellState::Hidden = cell.state {
                        self.reveal(self.cursor.0, self.cursor.1);
                    }
                }
                Key::Char('f') | Key::Char('F') => {
                    let cell = {
                        let i = self.position_index(self.cursor.0, self.cursor.1);
                        &mut self.cells[i]
                    };

                    match cell.state {
                        CellState::Hidden => cell.state = CellState::Flagged,
                        CellState::Flagged => cell.state = CellState::Hidden,
                        _ => (),
                    }
                }
                // Key::Char('r') => *self = Game::new(self.input, self.output, 1, self.width, self.height),
                Key::Char('q') => self.quit(),
                _ => (),
            }

            // reset cursor pos to the current pos
            write!(
                self.output,
                "{}",
                termion::cursor::Goto(self.cursor.0 as u16 + 2, self.cursor.1 as u16 + 2)
            )
            .unwrap();
            self.output.flush().unwrap();
        }
    }

    fn reveal(&mut self, x: usize, y: usize) {
        let mut cell = {
            let i = self.position_index(x, y);
            self.cells[i]
        };

        if let CellState::Revealed = cell.state {
            return;
        } else {
            let i = self.position_index(x, y);
            self.cells[i].state = CellState::Revealed;
            cell.state = CellState::Revealed;
        }

        if cell.mine {
            self.game_over();
            return;
        }

        let neighbors = self.neighbors(x, y);
        let surrounding_mines: u8 = neighbors
            .iter()
            .filter_map(|(x, y)| {
                let i = self.position_index(*x, *y);
                if self.cells[i].mine {
                    Some(1)
                } else {
                    None
                }
            })
            .sum();

        if surrounding_mines > 0 {
            write!(
                self.output,
                "{}{}{}",
                termion::cursor::Goto(x as u16 + 2, y as u16 + 2),
                surrounding_mines,
                style::Reset
            )
            .unwrap();
        } else {
            write!(
                self.output,
                "{}{}{}",
                termion::cursor::Goto(x as u16 + 2, y as u16 + 2),
                cell.as_char(),
                style::Reset,
            )
            .unwrap();

            for (x, y) in neighbors {
                self.reveal(x, y);
            }
        }
    }

    fn quit(&mut self) {
        write!(self.output, "{}", clear::All).unwrap();
        std::process::exit(0);
    }

    fn game_over(&mut self) {
        write!(self.output, "{}{}", termion::clear::All, GAME_OVER).unwrap();
        self.output.flush().unwrap();
        match self.input.next().unwrap().unwrap() {
            Key::Char('q') => self.quit(),
            // Key::Char('r') => *self = Game::new(self.input, self.output, 1, self.width, self.height),
            _ => (),
        }
    }
}

fn main() {
    let stderr = io::stderr();
    let mut stderr = stderr.lock();

    let mut args = env::args().skip(1);
    let mut width: Option<usize> = None;
    let mut height: Option<usize> = None;
    let mut difficulty: Option<u8> = None;

    println!("{:?}", args);
    loop {
        let arg = if let Some(arg) = args.next() {
            arg
        } else {
            break;
        };

        match arg.as_str() {
            "--help" => {
                print!("{}", HELP_TEXT);
                std::process::exit(0);
            }

            "-w" | "--width" => {
                if width.is_none() {
                    width = Some(
                        args.next()
                            .unwrap_or_else(|| {
                                stderr.write(b"no width given!").unwrap();
                                stderr.flush().unwrap();
                                std::process::exit(1);
                            })
                            .parse()
                            .unwrap_or_else(|_| {
                                stderr.write(b"invalid number given as width!").unwrap();
                                stderr.flush().unwrap();
                                std::process::exit(1);
                            }),
                    );
                }
            }

            "-h" | "--height" => {
                if height.is_none() {
                    height = Some(
                        args.next()
                            .unwrap_or_else(|| {
                                stderr.write(b"no height given!").unwrap();
                                stderr.flush().unwrap();
                                std::process::exit(1);
                            })
                            .parse()
                            .unwrap_or_else(|_| {
                                stderr.write(b"invalid number given as height!").unwrap();
                                stderr.flush().unwrap();
                                std::process::exit(1);
                            }),
                    );
                }
            }

            "-d" | "--difficulty" => {
                if difficulty.is_none() {
                    difficulty = match args
                        .next()
                        .unwrap_or_else(|| {
                            stderr.write(b"no difficulty given!").unwrap();
                            stderr.flush().unwrap();
                            std::process::exit(1);
                        })
                        .parse::<u8>()
                    {
                        Ok(n @ 0..=2) => Some(n),
                        _ => {
                            stderr
                                .write(b"invalid number given as difficulty!")
                                .unwrap();
                            stderr.flush().unwrap();
                            std::process::exit(1);
                        }
                    }
                }
            }

            _ => (),
        }
    }

    let stdin = io::stdin();
    let stdin = stdin.lock();
    let stdin = stdin.keys();
    let stdout = io::stdout();
    let stdout = stdout.lock().into_raw_mode().unwrap();

    let mut game = Game::new(
        stdin,
        stdout,
        difficulty.unwrap_or(1),
        width.unwrap(),
        height.unwrap(),
    );

    game.run();
}
