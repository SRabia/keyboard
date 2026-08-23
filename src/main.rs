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

#[derive(Debug, embedded_cli::Command)]
enum BaseCommand {
    /// Control RED Led
    Led {
        #[command(subcommand)]
        cmd: LedCommand,
    },
    /// Show some status
    Status,
}

#[derive(Debug, embedded_cli::Command)]
enum LedCommand {
    ///Turn led On
    TurnOn,
    ///Turn Led off
    TurnOff,
}
fn set_led(
    cli: &mut embedded_cli::cli::CliHandle<'_, Writer, EndpointError>,
    cmd: LedCommand,
) -> Result<(), EndpointError> {
    Ok(())
}

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
            let _ = shell(&mut class).await;
            log::info!("Disconnected fu");
        }
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, join(shell_fut, log_fut)).await;
}

struct Writer<'a> {
    usbcdc: CdcAcmClass<'a, Driver<'a, USB>>,
}

impl<'a> embedded_io_async::ErrorType for Writer<'a> {
    type Error = embassy_usb::driver::EndpointError;
}

impl<'a> embedded_io_async::Write for Writer<'a> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.usbcdc.write_packet(buf).await?;
        Ok(buf.len())
    }
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
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

async fn shell<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Disconnected> {
    let mut buf = [0; 64];
    loop {
        let n = class.read_packet(&mut buf).await?;
        let data = &buf[..n];
        class.write_packet(data).await?;
        match &buf[..n] {
            b"help" => {
                class.write_packet(b"Command help exe\n").await?;
            }
            _ => {
                class.write_packet(b"unknow command\n").await?;
            }
        }

        log::info!("data: {:x?}", data);
    }
}
