use std::env;
use std::io::{self, Write};
use std::time::Instant;
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

#[derive(Eq, PartialEq, Clone, Copy)]
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

    pub fn color(&self) -> &termion::color::Color {
        use termion::color;
        match self.state {
            CellState::Hidden => &color::LightBlue,
            CellState::Flagged => &color::Blue,
            CellState::Revealed => &color::Reset,
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
    start_time: Instant,
    difficulty: u8,
}

impl<R, W: Write> Game<R, W> {
    pub fn new(input: R, output: W, difficulty: u8, width: usize, height: usize) -> Self {
        Game {
            input,
            output,
            cells: Self::gen_board(difficulty, width, height),
            width,
            height,
            cursor: (0, 0),
            start_time: Instant::now(),
            difficulty,
        }
    }

    fn gen_board(difficulty: u8, width: usize, height: usize) -> Vec<Cell> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut cells = Vec::with_capacity(width * height);
        for _ in 0..width * height {
            cells.push(Cell {
                mine: rng.gen_range(difficulty, 30) < 3,
                state: CellState::Hidden,
            });
        }

        cells
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
        write!(self.output, "{}{}{}{}", Goto(1, 1), style::Reset, clear::All, CORNER_TL).unwrap();

        // draw top border
        for x in 2..self.width + 2 {
            write!(self.output, "{}{}", Goto(x as u16, 1), BORDER_HORIZONTAL).unwrap();
        }

        write!(
            self.output,
            "{}{}",
            Goto(self.width as u16 + 2, 1),
            CORNER_TR
        )
        .unwrap();

        // draw cells
        for y in 0..self.height {
            write!(self.output, "{}{}", Goto(1, y as u16 + 2), BORDER_VERTICAL).unwrap();

            for x in 0..self.width {
                let i = self.position_index(x, y);
                write!(
                    self.output,
                    "{}{}{}",
                    Goto(x as u16 + 2, y as u16 + 2),
                    termion::color::Fg(self.cells[i].color()),
                    self.cells[i].as_char(),
                )
                .unwrap();
            }

            write!(
                self.output,
                "{}{}{}",
                termion::color::Fg(termion::color::Reset),
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

    fn did_win(&self) -> bool {
        self.cells
            .iter()
            .filter(|cell| cell.mine)
            .all(|cell| cell.state == CellState::Flagged)
    }

    fn quit(&mut self) {
        write!(self.output, "{}{}", clear::All, termion::cursor::Goto(1, 1)).unwrap();
        self.output.flush().unwrap();
        std::process::exit(0);
    }
}

impl<R: Iterator<Item = Result<Key, io::Error>>, W: Write> Game<R, W> {
    fn run(&mut self) {
        write!(self.output, "{}", clear::All).unwrap();

        self.redraw();

        loop {
            let key = self
                .input
                .next()
                .expect("input.next() was None!")
                .expect("io error occurred!");

            match key {
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

                    write!(
                        self.output,
                        "{}{}{}",
                        termion::cursor::Goto(self.cursor.0 as u16 + 2, self.cursor.1 as u16 + 2),
                        termion::color::Fg(cell.color()),
                        cell.as_char()
                    )
                    .unwrap();
                }
                Key::Char('r') => {
                    self.cells = Self::gen_board(self.difficulty, self.width, self.height);
                    self.redraw();
                }
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

            if self.did_win() {
                self.win_game();
            }
        }
    }

    fn reveal(&mut self, x: usize, y: usize) {
        let i = self.position_index(x, y);

        match self.cells[i].state {
            CellState::Hidden if self.cells[i].mine => self.game_over(),
            CellState::Hidden if !self.cells[i].mine => {
                let i = self.position_index(x, y);
                self.cells[i].state = CellState::Revealed;

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
                        "{}{}{}{}",
                        termion::cursor::Goto(x as u16 + 2, y as u16 + 2),
                        termion::color::Fg(termion::color::Reset),
                        surrounding_mines,
                        style::Reset
                    )
                    .unwrap();
                } else {
                    write!(
                        self.output,
                        "{}{}{}{}",
                        termion::cursor::Goto(x as u16 + 2, y as u16 + 2),
                        termion::color::Fg(self.cells[i].color()),
                        self.cells[i].as_char(),
                        style::Reset,
                    )
                    .unwrap();

                    for (x, y) in neighbors {
                        self.reveal(x, y);
                    }
                }
            }
            _ => (),
        }
    }

    fn win_game(&mut self) {
        write!(self.output, "{}{}", clear::All, style::Reset).unwrap();
        draw_textbox(
            &mut self.output,
            (1, 1),
            &format!(
                "You Won!\n time: {} seconds \n\nreplay: r\nquit: q",
                self.start_time.elapsed().as_secs()
            ),
        );
        self.output.flush().unwrap();
        loop {
            let key = self.input.next().unwrap().unwrap();
            match key {
                Key::Char('r') => {
                    self.cells = Self::gen_board(self.difficulty, self.width, self.height);
                    self.redraw();
                    break;
                }
                Key::Char('q') => self.quit(),
                _ => (),
            }
        }
    }

    fn game_over(&mut self) {
        write!(self.output, "{}{}", clear::All, style::Reset).unwrap();
        draw_textbox(&mut self.output, (1, 1), " Game Over! \n\nretry: r\nquit:q");
        self.output.flush().unwrap();
        loop {
            match self.input.next().unwrap().unwrap() {
                Key::Char('q') => self.quit(),
                Key::Char('r') => {
                    self.cells = Self::gen_board(self.difficulty, self.width, self.height);
                    self.redraw();
                    break;
                }
                _ => (),
            }
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

/// draw a textbox
/// ex:
/// ┌──────────────┐
/// │  Game Over!  │
/// │              │
/// │  restart: r  │
/// │  quit: q     │
/// └──────────────┘
/// NOTE: a `pos` of (1, 1) is the top left of the screen
/// NOTE: centers text
fn draw_textbox<W: Write>(output: &mut W, pos: (u16, u16), text: &str) {
    let lines = text.lines().collect::<Vec<_>>();
    let max_width = lines
        .iter()
        .map(|line| line.len())
        .max()
        .expect("you need to have some text to draw!");
    let lines = lines.iter().map(|line| {
        format!(
            "{}{:^width$}{}",
            BORDER_VERTICAL,
            line,
            BORDER_VERTICAL,
            width = max_width
        )
    });
    let top_border = format!(
        "{}{:─<width$}{}",
        CORNER_TL,
        "",
        CORNER_TR,
        width = max_width
    );
    let bottom_border = format!(
        "{}{:─<width$}{}",
        CORNER_BL,
        "",
        CORNER_BR,
        width = max_width
    );

    write!(
        output,
        "{}{}",
        termion::cursor::Goto(pos.0, pos.1),
        top_border
    )
    .unwrap();
    for (i, line) in lines.clone().enumerate() {
        write!(
            output,
            "{}{}",
            termion::cursor::Goto(pos.0, pos.1 + 1 + i as u16),
            line
        )
        .unwrap();
    }
    write!(
        output,
        "{}{}",
        termion::cursor::Goto(pos.0, pos.1 + lines.len() as u16 + 1),
        bottom_border
    )
    .unwrap();
}
