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

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
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
    line: heapless::Vec<u8, 512>,
}

impl Terminal {
    //TODO: init with initial prompt line
    pub fn new() -> Self {
        Terminal {
            state: TerminalState::Normal,
            line: heapless::Vec::from_slice(b"> ").unwrap(),
        }
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

    fn process_esc(&mut self, c: u8) {
        //TODO: replace comment command with enum of value instead of hardcoded ascii code
        match c {
            b'A' => {
                //up
            }
            b'B' => {
                //down
            }
            b'D' => {
                //down
            }
            b'H' => {
                //down
            }
            b'F' => {
                //down
            }
            _ => {
                //echo back
            }
        }
        self.state = TerminalState::Normal;
    }

    //TODO: don't hardcode Vec size
    pub fn get_render_line(&mut self) -> heapless::Vec<u8, 64> {
        let mut render_line = heapless::Vec::new();
        render_line.extend_from_slice(self.line.as_slice()).unwrap();
        self.line.clear();
        //self.line.extend_from_slice(b"> ").unwrap();
        render_line
    }

    //TODO: replace comment command with enum of value instead of hardcoded ascii code

    fn process_normal(&mut self, c: u8) {
        match c {
            0x08 | 0x7f => {
                //backspace
                self.line.extend_from_slice(b"\x08 \x08").unwrap();
            }
            b'\r' | b'\n' => {
                self.line.extend_from_slice(b"\r\n> ").unwrap();
                //enter
            }
            _ => {
                self.line.push(c).unwrap();
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
        class
            .write_packet(terminal.get_render_line().as_slice())
            .await?;

        //for c in data {
        //    match c {
        //        0x08 | 0x7f => {
        //            //backspace
        //            class.write_packet(b"\x08 \x08").await?;
        //        }
        //        _ => {
        //            let echo: &[u8; 1] = &[*c];
        //            class.write_packet(echo).await?;
        //        }
        //    }
        //}
        //match &buf[..n] {
        //    b"help" => {
        //        class.write_packet(b"Command help exe\n").await?;
        //    }
        //    _ => {
        //        class.write_packet(b"unknow command\n").await?;
        //    }
        //}
    }
}
