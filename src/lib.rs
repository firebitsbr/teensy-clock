#![feature(lang_items)]
#![no_std]

extern crate gregor;

#[allow(dead_code, non_snake_case, non_camel_case_types, non_upper_case_globals, improper_ctypes)]
mod bindings;

#[macro_use]
mod serial;

use bindings::SPIClass;
use gregor::{DateTime, Utc, Month};
use serial::Serial;

const LED_PIN: u8 = 13;
const SQUARE_WAVE_PIN: u8 = 10;
const SPI_CHIP_SELECT_PIN: u8 = 6;
const SPI_MOSI_PIN: u8 = 7;
const SPI_MISO_PIN: u8 = 8;
const SPI_SCK_PIN: u8 = 14;

#[no_mangle]
pub extern fn rust_init() {
    unsafe {
        bindings::pinMode(LED_PIN, bindings::OUTPUT as u8);
        bindings::pinMode(SQUARE_WAVE_PIN, bindings::INPUT_PULLUP as u8);

        bindings::pinMode(SPI_CHIP_SELECT_PIN, bindings::OUTPUT as u8);
        SPIClass::setMOSI(SPI_MOSI_PIN);
        SPIClass::setMISO(SPI_MISO_PIN);
        SPIClass::setSCK(SPI_SCK_PIN);
        SPIClass::begin();
        SPIClass::setBitOrder(bindings::MSBFIRST as u8);
        SPIClass::setDataMode(bindings::SPI_MODE1 as u8);
        bindings::digitalWrite(SPI_CHIP_SELECT_PIN, bindings::LOW as u8);
        SPIClass::transfer(0x8E);
        SPIClass::transfer(0x20);
        bindings::digitalWrite(SPI_CHIP_SELECT_PIN, bindings::HIGH as u8);
    }
}

#[no_mangle]
pub extern fn read_int(delimiter: u8) -> u32 {
    Serial.try_read_int_until(delimiter).unwrap()
}

fn spi_read(address: u8, data: &mut [u8]) {
    unsafe {
        bindings::digitalWrite(SPI_CHIP_SELECT_PIN, bindings::LOW as u8);
        SPIClass::transfer(address);
        for byte in data {
            *byte = SPIClass::transfer(0);
        }
        bindings::digitalWrite(SPI_CHIP_SELECT_PIN, bindings::HIGH as u8);
    }
}

fn spi_write(address: u8, data: &[u8]) {
    unsafe {
        bindings::digitalWrite(SPI_CHIP_SELECT_PIN, bindings::LOW as u8);
        SPIClass::transfer(address);
        for &byte in data {
            SPIClass::transfer(byte);
        }
        bindings::digitalWrite(SPI_CHIP_SELECT_PIN, bindings::HIGH as u8);
    }
}

/// Binary-Coded Decimal
fn bcd_decode(n: u8) -> u8 {
    (n >> 4) * 10 + (n & 0xF)
}

fn bcd_encode(n: u8) -> u8 {
    assert!(n < 100);
    (n / 10) << 4 | (n % 10)
}

fn rtc_get() -> DateTime<Utc> {
    let mut data = [0, 0, 0, 0, 0, 0, 0];
    spi_read(0x00, &mut data);
    let second = bcd_decode(data[0]);
    let minute = bcd_decode(data[1]);
    let hour = bcd_decode(data[2]);
    // data[3] is the day of the week, but we don’t rely on the RTC for that.
    let day = bcd_decode(data[4]);
    let month = Month::from_number(bcd_decode(data[5])).unwrap();
    let year = 2000 + i32::from(bcd_decode(data[6]));
    DateTime::new(Utc, year, month, day, hour, minute, second)
}

fn rtc_set(datetime: &DateTime<Utc>) {
    spi_write(0x80, &[
        bcd_encode(datetime.second()),
        bcd_encode(datetime.minute()),
        bcd_encode(datetime.hour()),
        0,  // Day of the week, unused
        bcd_encode(datetime.day()),
        bcd_encode(datetime.month().to_number()),
        bcd_encode((datetime.year() - 2000) as u8),
    ])
}

#[no_mangle]
pub extern fn rtc_print() {
    println!("Current RTC datetime: {:?}", rtc_get());
}

#[no_mangle]
pub extern fn rtc_sync() {
    let year = read_int(b'-') as i32;
    let month = Month::from_number(read_int(b'-') as u8).unwrap();
    let day = read_int(b' ') as u8;
    let hour = read_int(b':') as u8;
    let minute = read_int(b':') as u8;
    let second = read_int(b'\n') as u8;
    rtc_set(&DateTime::new(Utc, year, month, day, hour, minute, second))
}

mod std {
    pub use core::*;
    pub mod os {
        #[allow(non_camel_case_types, dead_code)]
        pub mod raw {
            pub enum c_void {}
            pub type c_uchar = u8;
            pub type c_short = i16;
            pub type c_ushort = u16;
            pub type c_int = i32;
            pub type c_uint = u32;
            pub type c_long = i32;
            pub type c_ulong = u32;
            pub type c_longlong = i64;
            pub type c_ulonglong = u64;
        }
    }
}

#[lang = "panic_fmt"]
pub extern fn rust_begin_panic(msg: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    println!("Panic at {}:{}, {}", file, line, msg);
    loop {}
}

#[lang = "eh_personality"]
pub extern fn rust_eh_personality() {}
