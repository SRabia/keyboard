#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
//use embassy_rp::peripherals::USB;
//use embassy_rp::usb::{Driver, Instance, InterruptHandler};
//use embassy_rp::{bind_interrupts, Peri};
use embassy_time::{Duration, Timer};

//use static_cell::StaticCell;
use {defmt_embassy_usbserial as _, panic_probe as _};

//bind_interrupts!(struct Irqs{
//    USBCTRL_IRQ => InterruptHandler<USB>;
//});

//i2c: twim::Twim<'static, embassy_nrf::peripherals::TWISPI0>
//#[embassy_executor::task]
//async fn defmt_usb_task(usb: Peri<'static, embassy_rp::peripherals::USB>) {
//    let driver = embassy_rp::usb::Driver::new(usb, Irqs);
//    let usb_config = {
//        let mut c = embassy_usb::Config::new(0x1234, 0x5678);
//        c.serial_number = Some("defmt");
//        c.max_packet_size_0 = 64;
//        c.composite_with_iads = true;
//        c.device_class = 0xEF;
//        c.device_sub_class = 0x02;
//        c.device_protocol = 0x01;
//        c
//    };
//    defmt_embassy_usbserial::run(driver, usb_config).await;
//}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Automatically configures standard RP2040 clocks and peripherals
    let p = embassy_rp::init(Default::default());

    //spawner.must_spawn(defmt_usb_task(p.USB));
    info!("Pro Micro RP2040 initialized with Embassy!");

    // PIN 25 maps directly to the onboard WS2812 NeoPixel data line
    let mut led = Output::new(p.PIN_17, Level::Low);
    led.set_high();

    loop {
        info!("Tick");

        // Asynchronously sleep without locking up the CPU core
        Timer::after(Duration::from_millis(1000)).await;
    }
}
