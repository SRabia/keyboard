#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_time::{Duration, Timer};

use {embassy_usb_logger as _, panic_probe as _};

bind_interrupts!(struct Irqs{
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let driver = Driver::new(p.USB, Irqs);
    spawner.spawn(logger_task(driver)).unwrap();

    let mut led = Output::new(p.PIN_17, Level::Low);

    log::info!("Pro Micro RP2040 initialized with Embassy!");
    Timer::after(Duration::from_millis(10000)).await;
    loop {
        led.set_high();
        Timer::after(Duration::from_millis(50)).await;
        led.set_low();
        // Asynchronously sleep without locking up the CPU core
        Timer::after(Duration::from_millis(2000)).await;
    }
}
