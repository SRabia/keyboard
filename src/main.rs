#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::gpio::{AnyPin, Level, Output};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, Instance, InterruptHandler};
use embassy_rp::{bind_interrupts, Peri};
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Config};
use log as _;
use panic_probe as _;

use keyboard::terminal;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn heartbeat(led: Peri<'static, AnyPin>) {
    let mut output_led = Output::new(led, Level::Low);
    loop {
        output_led.toggle();
        Timer::after_millis(50).await;
        output_led.toggle();
        Timer::after_secs(3).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);
    let mut config = Config::new(keyboard::config::usb::PID, keyboard::config::usb::VID);
    config.manufacturer = Some(keyboard::config::usb::MAN);
    config.product = Some("USB-serial example");
    config.serial_number = Some("12345678");
    config.max_power = keyboard::config::usb::MAX_POWER;
    config.max_packet_size_0 = keyboard::config::usb::MAX_PACKET_SIZE_0 as u8;

    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; keyboard::config::usb::MAX_PACKET_SIZE_0];

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

    spawner.spawn(heartbeat(p.PIN_17.into())).unwrap();

    let mut class = CdcAcmClass::new(
        &mut builder,
        &mut state,
        keyboard::config::usb::MAX_PACKET_SIZE_0 as u16,
    );

    let logger_class = CdcAcmClass::new(
        &mut builder,
        &mut logger_state,
        keyboard::config::usb::MAX_PACKET_SIZE_0 as u16,
    );

    let log_fut = embassy_usb_logger::with_class!(
        { keyboard::config::LOGGER_BUFFER_MAX },
        log::LevelFilter::Info,
        logger_class
    );

    let mut usb = builder.build();

    let usb_fut = usb.run();
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

async fn terminal_task<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Disconnected> {
    let mut buf = [0; keyboard::config::usb::MAX_PACKET_SIZE_0];
    let mut term = terminal::Terminal::new();
    loop {
        let n = class.read_packet(&mut buf).await?;
        let data = &buf[..n];
        term.update(data);
        if n > 0 {
            let seg_write_64 = term.take_output();
            for s in seg_write_64.chunks(keyboard::config::usb::MAX_PACKET_SIZE_0) {
                let _ = class
                    .write_packet(s)
                    .await
                    .map_err(|e| log::info!("fail to write packed usb {e}"));
            }
        }
    }
}
