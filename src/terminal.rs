use core::str::FromStr;

const MAX_BUF_OUT_LINE: usize = 1024;
const MAX_BUF_IN_LINE: usize = 258;

enum TerminalState {
    Normal,
    Esc,
    EscBracket,
}

pub struct Terminal {
    state: TerminalState,
    pub line_input: heapless::Vec<u8, MAX_BUF_IN_LINE>,
    init_prompt: heapless::String<128>,
    pub line_output: heapless::Vec<u8, MAX_BUF_OUT_LINE>,
    pub cursor: usize,
}

impl Default for Terminal {
    fn default() -> Self {
        Terminal::new()
    }
}

impl Terminal {
    //TODO: init with initial prompt line
    pub fn new() -> Self {
        Terminal {
            state: TerminalState::Normal,
            line_output: heapless::Vec::from_slice(b"> ").unwrap(),
            line_input: heapless::Vec::new(),
            init_prompt: heapless::String::from_str(">").unwrap(),
            cursor: 0,
        }
    }

    pub fn with_prompt(mut self, prompt: &str) -> Self {
        self.init_prompt = heapless::String::from_str(prompt).unwrap();
        self
    }

    pub fn update(&mut self, raw_line: &[u8]) {
        for c in raw_line {
            match self.state {
                TerminalState::Normal => self.process_normal(*c),
                TerminalState::Esc => {
                    if *c == b'[' {
                        self.state = TerminalState::EscBracket
                    } else {
                        self.state = TerminalState::Normal
                    }
                }
                TerminalState::EscBracket => self.process_esc(*c),
            }
        }
    }

    fn move_left(&mut self, amount: usize) {
        for _ in 0..amount {
            self.output(b"\x1b[D");
        }
    }

    fn prompt(&mut self) {
        self.output(self.init_prompt.clone().as_bytes());
    }

    fn enter(&mut self) {
        self.output(b"\r\n");
        self.prompt();
        self.cursor = 0;
        self.line_input.clear();
        //TODO: add history here
    }

    fn _move_right(&mut self, amount: usize) {
        for _ in 0..amount {
            self.output(b"\x1b[C");
        }
    }

    fn output(&mut self, data: &[u8]) {
        let _ = self
            .line_output
            .extend_from_slice(data)
            .map_err(|e| log::info!("output extend fail with erro {e}"));
    }

    fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.output(b"\x1b[D");
        }
    }

    fn move_cursor_right(&mut self) {
        // Right
        if self.cursor < self.line_input.len() {
            self.cursor += 1;
            self.output(b"\x1b[C");
        }
    }

    fn process_esc(&mut self, c: u8) {
        //TODO: replace comment command with enum of value instead of hardcoded ascii code
        match c {
            b'A' => {
                //up history
            }
            b'B' => {
                //down history
            }
            b'C' => {
                self.move_cursor_right();
            }
            b'D' => {
                self.move_cursor_left();
            }
            b'H' => {
                //Home
            }
            b'F' => {
                //End
            }
            //TODO: have <C-a> to go home
            //TODO: have <C-e> to go end
            //TODO: have <C-u> to delete all before cursor
            _ => {}
        }
        self.state = TerminalState::Normal;
    }

    //TODO: don't hardcode Vec size
    pub fn take_output(&mut self) -> heapless::Vec<u8, MAX_BUF_OUT_LINE> {
        let mut output = heapless::Vec::new();
        core::mem::swap(&mut output, &mut self.line_output);
        output
    }

    fn redraw_from_cursor(&mut self) {
        self.output(b"\r");
        self.prompt();
        //reprint the prompt
        let input = self.line_input.clone();
        //log::info!("redraw from cursor len input {}", self.line_input.len());
        self.output(input.as_slice());
        self.output(b"\x1b[K");
        let amount = self.line_input.len() - self.cursor;
        self.move_left(amount);
    }

    fn insert_character(&mut self, c: u8) {
        if self.cursor == self.line_input.len() {
            let _ = self
                .line_input
                .push(c)
                .map_err(|e| log::info!("push char extend fail with erro {e}"));
            self.output(&[c]);
            self.cursor += 1;
            return;
        }
        let _ = self
            .line_input
            .insert(self.cursor, c)
            .map_err(|e| log::info!("insert char extend fail with erro {e}"));
        self.cursor += 1;
        self.redraw_from_cursor();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        self.line_input.remove(self.cursor);
        self.redraw_from_cursor();
    }

    //TODO: replace comment command with enum of value instead of hardcoded ascii code
    fn process_normal(&mut self, c: u8) {
        match c {
            0x1b => self.state = TerminalState::Esc,
            0x08 | 0x7f => {
                self.backspace();
            }
            b'\r' | b'\n' => {
                self.enter();
            }
            _ => {
                self.insert_character(c);
            }
        }
    }
}
