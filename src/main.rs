#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, Instance, InterruptHandler};
use embassy_time::{Duration, Timer};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Config};
use log as _;
use log::info;
use panic_probe as _;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn heartbeat() {
    loop {
        log::info!("still alive");
        Timer::after_secs(10).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut led = Output::new(p.PIN_17, Level::Low);

    let driver = Driver::new(p.USB, Irqs);

    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("USB-serial example");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    let mut state = State::new();
    let mut logger_state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [], // no msos descriptors
        &mut control_buf,
    );
    led.set_high();

    spawner.spawn(heartbeat()).unwrap();

    // Create classes on the builder.
    let mut class = CdcAcmClass::new(&mut builder, &mut state, 64);

    // Create a class for the logger
    let logger_class = CdcAcmClass::new(&mut builder, &mut logger_state, 64);

    // Creates the logger and returns the logger future
    // Note: You'll need to use log::info! afterwards instead of info! for this to work (this also applies to all the other log::* macros)
    let log_fut = embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, logger_class);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();
    Timer::after(Duration::from_millis(2000)).await;

    info!("Hello there!");
    // Do stuff with the class!
    let shell_fut = async {
        loop {
            class.wait_connection().await;
            log::info!("Connected me");
            let _ = terminal_task(&mut class).await;
            log::info!("Disconnected fu");
        }
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, join(shell_fut, log_fut)).await;
}

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

enum TerminalState {
    Normal,
    Esc,
    EscBracket,
}

struct Terminal {
    state: TerminalState,
    line_input: heapless::Vec<u8, 128>,
    line_output: heapless::Vec<u8, 256>,
    cursor: usize,
}

impl Terminal {
    //TODO: init with initial prompt line
    pub fn new() -> Self {
        Terminal {
            state: TerminalState::Normal,
            line_output: heapless::Vec::from_slice(b"> ").unwrap(),
            line_input: heapless::Vec::new(),
            cursor: 0,
        }
    }

    pub fn update(&mut self, raw_line: &[u8]) {
        log::info!("usb packet receive {:?}", raw_line);
        for c in raw_line {
            match self.state {
                TerminalState::Normal => self.process_normal(*c),
                TerminalState::Esc => {
                    log::info!("esc detected");
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
        log::info!("moved left");
    }
    fn enter(&mut self) {
        self.output(b"\r\n> ");
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
        log::info!("line buffer size {}", self.line_output.len());
    }

    fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.output(b"\x1b[D");
            log::info!(
                "move cursor left {} line_len {}",
                self.cursor,
                self.line_input.len()
            );
        }
    }
    fn move_cursor_right(&mut self) {
        // Right
        if self.cursor < self.line_input.len() {
            self.cursor += 1;
            self.output(b"\x1b[C");
            log::info!(
                "move cursor right {} line_len {}",
                self.cursor,
                self.line_input.len()
            );
        }
    }

    fn process_esc(&mut self, c: u8) {
        log::info!("esc process");
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
            _ => {}
        }
        self.state = TerminalState::Normal;
    }

    //TODO: don't hardcode Vec size
    pub fn take_output(&mut self) -> heapless::Vec<u8, 256> {
        let mut output = heapless::Vec::new();
        core::mem::swap(&mut output, &mut self.line_output);
        output
    }

    fn redraw_from_cursor(&mut self) {
        self.output(b"\r");
        self.output(b"> ");
        //reprint the prompt
        let input = self.line_input.clone();
        log::info!("redraw from cursor len input {}", self.line_input.len());
        self.output(input.as_slice());
        self.output(b"\x1b[K");
        let amount = self.line_input.len() - self.cursor;
        log::info!("moving left by {amount}");
        self.move_left(amount);
    }

    fn insert_character(&mut self, c: u8) {
        log::info!("assert {} <= {}", self.cursor, self.line_input.len());
        assert!(self.cursor <= self.line_input.len());
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
        log::info!("process normal");
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

async fn terminal_task<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Disconnected> {
    let mut buf = [0; 64];
    let mut terminal = Terminal::new();
    loop {
        let n = class.read_packet(&mut buf).await?;
        let data = &buf[..n];
        terminal.update(data);
        if n > 0 {
            let seg_write_64 = terminal.take_output();
            for s in seg_write_64.chunks(2) {
                let _ = class
                    .write_packet(s)
                    .await
                    .map_err(|e| log::info!("fail to write packed usb {e}"));
            }
        }
    }
}
