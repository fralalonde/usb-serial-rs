//! USB <-> USART serial bridge for Adafruit Trinket M0
//! Duplex, fixed function, unbuffered
//! Blocking UART writes take place in USB IRQ context, not great but acts as rate limiter
//! A better version would buffer UART writes, toggling DRE irq enable for fully async operation
//! Also, this is still using atsamd_hal::uart::v1 because v2 writes seemed broken. Yes, I should have filed an issue.
//!
//! Serial parameters appear in USB Dev Desc: (921600 bps, 8N1, 0-3V, pins D3:RX D4:TX)

#![no_std]
#![no_main]

use panic_rtt_target as _;

// use panic_probe as _;
// use defmt_rtt as _;

use trinket_m0 as bsp;
use bsp::hal;

use hal::sercom::UART0;
use hal::pac::{interrupt, CorePeripherals, Peripherals};

use hal::usb::UsbBus;

use hal::gpio::{*};

use hal::sercom::*;

use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::{UsbDeviceBuilder, UsbVidPid, UsbDevice};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use cortex_m::prelude::*;
use cortex_m::asm::delay as cycle_delay;
use cortex_m::peripheral::NVIC;

use hal::thumbv6m::clock::GenericClockController;
use crate::hal::time::Hertz;

static mut UART_SERIAL: Option<UART0<Sercom0Pad3<Pa7<PfD>>, Sercom0Pad2<Pa6<PfD>>, (), ()>> = None;

static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;
static mut USB_BUS: Option<UsbDevice<UsbBus>> = None;
static mut USB_SERIAL: Option<SerialPort<UsbBus>> = None;

const RXC: u8 = 0x04;

// defmt::timestamp!("{=u32:us}", {
//     0
// });

#[trinket_m0::entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut pins = bsp::Pins::new(peripherals.PORT);
    let mut red_led = pins.d13.into_open_drain_output(&mut pins.port);

    rtt_target::rtt_init_print!(NoBlockSkip, 1024);

    let bus_allocator = unsafe {
        USB_ALLOCATOR = Some(bsp::usb_allocator(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.PM,
            pins.usb_dm,
            pins.usb_dp,
        ));
        USB_ALLOCATOR.as_ref().unwrap()
    };

    let usb_serial = SerialPort::new(&bus_allocator);

    let bus = UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Mungo Machines")
        .product("Adafruit Trinket M0 - USB Serial Adapter (921600 bps, 8N1, 0-3V, pins D3:RX D4:TX)")
        .serial_number("P0RKCH0PSS4NDWICH3S")
        .device_class(USB_CLASS_CDC)
        .build();

    let mut serial: UART0<Sercom0Pad3<Pa7<PfD>>, Sercom0Pad2<Pa6<PfD>>, (), ()> = bsp::uart(
        &mut clocks,
        Hertz(921600),
        peripherals.SERCOM0,
        &mut peripherals.PM,
        pins.d3.into_floating_input(&mut pins.port),
        pins.d4.into_floating_input(&mut pins.port),
        &mut pins.port,
    );
    serial.intenset(|r| unsafe { r.bits(RXC); });

    unsafe {
        USB_SERIAL.replace(usb_serial);
        USB_BUS.replace(bus);
        UART_SERIAL.replace(serial);

        core.NVIC.set_priority(interrupt::USB, 1);
        NVIC::unmask(interrupt::USB);
        core.NVIC.set_priority(interrupt::SERCOM0, 1);
        NVIC::unmask(interrupt::SERCOM0);
    }

    loop {
        // BLINK IT LIKE YOU STOLE IT
        red_led.toggle();
        cycle_delay(25_000_000);
        // defmt::println!("blinky");
        rtt_target::rprintln!("blinky");
    }
}

#[interrupt]
fn USB() {
    let usb_dev = unsafe { USB_BUS.as_mut().unwrap() };
    let usb_serial = unsafe { USB_SERIAL.as_mut().unwrap() };
    usb_dev.poll(&mut [usb_serial]);
    let mut buf = [0u8; 64];
    if let Ok(count) = usb_serial.read(&mut buf) {
        if count == 0 { return; }
        let uart_serial = unsafe { UART_SERIAL.as_mut().unwrap() };
        for byte in &buf[0..count] {
            while let Err(_) = uart_serial.write(*byte) {}
        }
    }
}

#[interrupt]
fn SERCOM0() {
    let usb_serial = unsafe { USB_SERIAL.as_mut().unwrap() };
    let uart_serial = unsafe { UART_SERIAL.as_mut().unwrap() };
    if let Ok(byte) = uart_serial.read() {
        if let Err(e) = usb_serial.write(&[byte]) {
            // defmt::info!("USB write err {:?}", e);
            rtt_target::rprintln!("USB write err {:?}", e);
        }
    }
}







