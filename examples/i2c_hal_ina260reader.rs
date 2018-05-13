#![feature(used)]
#![no_main]
#![no_std]

#[macro_use(entry, exception)]
extern crate cortex_m_rt;

use cortex_m_rt::ExceptionFrame;

extern crate embedded_hal;
extern crate panic_abort;
use embedded_hal::blocking::i2c::Write;

extern crate cortex_m;
extern crate stm32f042_hal as hal;

extern crate numtoa;
use numtoa::NumToA;

use hal::i2c::*;
use hal::prelude::*;
use hal::stm32f042;

const SSD1306_BYTE_CMD: u8 = 0x00;
const SSD1306_BYTE_DATA: u8 = 0x40;
const SSD1306_BYTE_CMD_SINGLE: u8 = 0x80;

const SSD1306_DISPLAY_RAM: u8 = 0xA4;
const SSD1306_DISPLAY_NORMAL: u8 = 0xA6;
const SSD1306_DISPLAY_OFF: u8 = 0xAE;
const SSD1306_DISPLAY_ON: u8 = 0xAF;

const SSD1306_MEMORY_ADDR_MODE: u8 = 0x20;
const SSD1306_COLUMN_RANGE: u8 = 0x21;
const SSD1306_PAGE_RANGE: u8 = 0x22;

const SSD1306_DISPLAY_START_LINE: u8 = 0x40;
const SSD1306_SCAN_MODE_NORMAL: u8 = 0xC0;
const SSD1306_DISPLAY_OFFSET: u8 = 0xD3;
const SSD1306_PIN_MAP: u8 = 0xDA;

const SSD1306_DISPLAY_CLK_DIV: u8 = 0xD5;
const SSD1306_CHARGE_PUMP: u8 = 0x8D;

exception!(*, default_handler);

fn default_handler(_irqn: i16) {}

exception!(HardFault, hard_fault);

fn hard_fault(_ef: &ExceptionFrame) -> ! {
    loop {}
}

entry!(main);

fn main() -> ! {
    if let Some(p) = stm32f042::Peripherals::take() {
        let gpiof = p.GPIOF.split();
        let mut rcc = p.RCC.constrain();
        let _ = rcc.cfgr.freeze();

        let scl = gpiof
            .pf1
            .into_alternate_af1()
            .internal_pull_up(true)
            .set_open_drain();
        let sda = gpiof
            .pf0
            .into_alternate_af1()
            .internal_pull_up(true)
            .set_open_drain();

        /* Setup I2C1 */
        let mut i2c = I2c::i2c1(p.I2C1, (scl, sda), 10.khz());

        /* Initialise SSD1306 display */
        let _ = ssd1306_init(&mut i2c);

        /* Print a welcome message on the display */
        let _ = ssd1306_pos(&mut i2c, 0, 0);

        /* Endless loop */
        loop {
            let _ = ssd1306_pos(&mut i2c, 0, 0);
            let mut data = [0; 2];
            let _ = i2c.write_read(0x40, &[0x00], &mut data);
            let config = (u16::from(data[0]) << 8) | u16::from(data[1]);

            let mut buffer = [0u8; 10];
            let count_start = config.numtoa(10, &mut buffer);

            let _ = ssd1306_print_bytes(&mut i2c, &buffer[count_start..]);

            let _ = ssd1306_pos(&mut i2c, 0, 1);

            let mut data = [0; 2];
            let _ = i2c.write_read(0x40, &[0x02], &mut data);
            let voltage = ((u32::from(data[0]) << 8) | u32::from(data[1])) * 1250;

            let mut buffer = [0u8; 10];
            let count_start = voltage.numtoa(10, &mut buffer);
            let _ = ssd1306_print_bytes(&mut i2c, &buffer[count_start..]);
            let _ = ssd1306_print_bytes(&mut i2c, b"uV     ");

            let _ = ssd1306_pos(&mut i2c, 0, 2);

            let mut data = [0; 2];
            let _ = i2c.write_read(0x40, &[0x01], &mut data);
            let voltage = ((u32::from(data[0]) << 8) | u32::from(data[1])) * 1250;

            let mut buffer = [0u8; 10];
            let count_start = voltage.numtoa(10, &mut buffer);
            let _ = ssd1306_print_bytes(&mut i2c, &buffer[count_start..]);
            let _ = ssd1306_print_bytes(&mut i2c, b"uA     ");
        }
    }

    loop {}
}

/// Print characters on the display with the embedded 7x7 font
fn ssd1306_print_bytes<I2C, E>(i2c: &mut I2C, bytes: &[u8]) -> Result<(), E>
where
    I2C: Write<Error = E>,
{
    /* A 7x7 font shamelessly borrowed from https://github.com/techninja/MarioChron/ */
    const FONT_7X7: [u8; 672] = [
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00, // (space)
        0x00,
        0x00,
        0x5F,
        0x00,
        0x00,
        0x00,
        0x00, // !
        0x00,
        0x07,
        0x00,
        0x07,
        0x00,
        0x00,
        0x00, // "
        0x14,
        0x7F,
        0x14,
        0x7F,
        0x14,
        0x00,
        0x00, // #
        0x24,
        0x2A,
        0x7F,
        0x2A,
        0x12,
        0x00,
        0x00, // $
        0x23,
        0x13,
        0x08,
        0x64,
        0x62,
        0x00,
        0x00, // %
        0x36,
        0x49,
        0x55,
        0x22,
        0x50,
        0x00,
        0x00, // &
        0x00,
        0x05,
        0x03,
        0x00,
        0x00,
        0x00,
        0x00, // '
        0x00,
        0x1C,
        0x22,
        0x41,
        0x00,
        0x00,
        0x00, // (
        0x00,
        0x41,
        0x22,
        0x1C,
        0x00,
        0x00,
        0x00, // )
        0x08,
        0x2A,
        0x1C,
        0x2A,
        0x08,
        0x00,
        0x00, // *
        0x08,
        0x08,
        0x3E,
        0x08,
        0x08,
        0x00,
        0x00, // +
        0x00,
        0x50,
        0x30,
        0x00,
        0x00,
        0x00,
        0x00, // ,
        0x00,
        0x18,
        0x18,
        0x18,
        0x18,
        0x18,
        0x00, // -
        0x00,
        0x60,
        0x60,
        0x00,
        0x00,
        0x00,
        0x00, // .
        0x20,
        0x10,
        0x08,
        0x04,
        0x02,
        0x00,
        0x00, // /
        0x1C,
        0x3E,
        0x61,
        0x41,
        0x43,
        0x3E,
        0x1C, // 0
        0x40,
        0x42,
        0x7F,
        0x7F,
        0x40,
        0x40,
        0x00, // 1
        0x62,
        0x73,
        0x79,
        0x59,
        0x5D,
        0x4F,
        0x46, // 2
        0x20,
        0x61,
        0x49,
        0x4D,
        0x4F,
        0x7B,
        0x31, // 3
        0x18,
        0x1C,
        0x16,
        0x13,
        0x7F,
        0x7F,
        0x10, // 4
        0x27,
        0x67,
        0x45,
        0x45,
        0x45,
        0x7D,
        0x38, // 5
        0x3C,
        0x7E,
        0x4B,
        0x49,
        0x49,
        0x79,
        0x30, // 6
        0x03,
        0x03,
        0x71,
        0x79,
        0x0D,
        0x07,
        0x03, // 7
        0x36,
        0x7F,
        0x49,
        0x49,
        0x49,
        0x7F,
        0x36, // 8
        0x06,
        0x4F,
        0x49,
        0x49,
        0x69,
        0x3F,
        0x1E, // 9
        0x00,
        0x36,
        0x36,
        0x00,
        0x00,
        0x00,
        0x00, // :
        0x00,
        0x56,
        0x36,
        0x00,
        0x00,
        0x00,
        0x00, // ;
        0x00,
        0x08,
        0x14,
        0x22,
        0x41,
        0x00,
        0x00, // <
        0x14,
        0x14,
        0x14,
        0x14,
        0x14,
        0x00,
        0x00, // =
        0x41,
        0x22,
        0x14,
        0x08,
        0x00,
        0x00,
        0x00, // >
        0x02,
        0x01,
        0x51,
        0x09,
        0x06,
        0x00,
        0x00, // ?
        0x32,
        0x49,
        0x79,
        0x41,
        0x3E,
        0x00,
        0x00, // @
        0x7E,
        0x11,
        0x11,
        0x11,
        0x7E,
        0x00,
        0x00, // A
        0x7F,
        0x49,
        0x49,
        0x49,
        0x36,
        0x00,
        0x00, // B
        0x3E,
        0x41,
        0x41,
        0x41,
        0x22,
        0x00,
        0x00, // C
        0x7F,
        0x7F,
        0x41,
        0x41,
        0x63,
        0x3E,
        0x1C, // D
        0x7F,
        0x49,
        0x49,
        0x49,
        0x41,
        0x00,
        0x00, // E
        0x7F,
        0x09,
        0x09,
        0x01,
        0x01,
        0x00,
        0x00, // F
        0x3E,
        0x41,
        0x41,
        0x51,
        0x32,
        0x00,
        0x00, // G
        0x7F,
        0x08,
        0x08,
        0x08,
        0x7F,
        0x00,
        0x00, // H
        0x00,
        0x41,
        0x7F,
        0x41,
        0x00,
        0x00,
        0x00, // I
        0x20,
        0x40,
        0x41,
        0x3F,
        0x01,
        0x00,
        0x00, // J
        0x7F,
        0x08,
        0x14,
        0x22,
        0x41,
        0x00,
        0x00, // K
        0x7F,
        0x7F,
        0x40,
        0x40,
        0x40,
        0x40,
        0x00, // L
        0x7F,
        0x02,
        0x04,
        0x02,
        0x7F,
        0x00,
        0x00, // M
        0x7F,
        0x04,
        0x08,
        0x10,
        0x7F,
        0x00,
        0x00, // N
        0x3E,
        0x7F,
        0x41,
        0x41,
        0x41,
        0x7F,
        0x3E, // O
        0x7F,
        0x09,
        0x09,
        0x09,
        0x06,
        0x00,
        0x00, // P
        0x3E,
        0x41,
        0x51,
        0x21,
        0x5E,
        0x00,
        0x00, // Q
        0x7F,
        0x7F,
        0x11,
        0x31,
        0x79,
        0x6F,
        0x4E, // R
        0x46,
        0x49,
        0x49,
        0x49,
        0x31,
        0x00,
        0x00, // S
        0x01,
        0x01,
        0x7F,
        0x01,
        0x01,
        0x00,
        0x00, // T
        0x3F,
        0x40,
        0x40,
        0x40,
        0x3F,
        0x00,
        0x00, // U
        0x1F,
        0x20,
        0x40,
        0x20,
        0x1F,
        0x00,
        0x00, // V
        0x7F,
        0x7F,
        0x38,
        0x1C,
        0x38,
        0x7F,
        0x7F, // W
        0x63,
        0x14,
        0x08,
        0x14,
        0x63,
        0x00,
        0x00, // X
        0x03,
        0x04,
        0x78,
        0x04,
        0x03,
        0x00,
        0x00, // Y
        0x61,
        0x51,
        0x49,
        0x45,
        0x43,
        0x00,
        0x00, // Z
        0x00,
        0x00,
        0x7F,
        0x41,
        0x41,
        0x00,
        0x00, // [
        0x02,
        0x04,
        0x08,
        0x10,
        0x20,
        0x00,
        0x00, // "\"
        0x41,
        0x41,
        0x7F,
        0x00,
        0x00,
        0x00,
        0x00, // ]
        0x04,
        0x02,
        0x01,
        0x02,
        0x04,
        0x00,
        0x00, // ^
        0x40,
        0x40,
        0x40,
        0x40,
        0x40,
        0x00,
        0x00, // _
        0x00,
        0x01,
        0x02,
        0x04,
        0x00,
        0x00,
        0x00, // `
        0x20,
        0x54,
        0x54,
        0x54,
        0x78,
        0x00,
        0x00, // a
        0x7F,
        0x48,
        0x44,
        0x44,
        0x38,
        0x00,
        0x00, // b
        0x38,
        0x44,
        0x44,
        0x44,
        0x20,
        0x00,
        0x00, // c
        0x38,
        0x44,
        0x44,
        0x48,
        0x7F,
        0x00,
        0x00, // d
        0x38,
        0x54,
        0x54,
        0x54,
        0x18,
        0x00,
        0x00, // e
        0x08,
        0x7E,
        0x09,
        0x01,
        0x02,
        0x00,
        0x00, // f
        0x08,
        0x14,
        0x54,
        0x54,
        0x3C,
        0x00,
        0x00, // g
        0x7F,
        0x08,
        0x04,
        0x04,
        0x78,
        0x00,
        0x00, // h
        0x00,
        0x44,
        0x7D,
        0x40,
        0x00,
        0x00,
        0x00, // i
        0x20,
        0x40,
        0x44,
        0x3D,
        0x00,
        0x00,
        0x00, // j
        0x00,
        0x7F,
        0x10,
        0x28,
        0x44,
        0x00,
        0x00, // k
        0x00,
        0x41,
        0x7F,
        0x40,
        0x00,
        0x00,
        0x00, // l
        0x7C,
        0x04,
        0x18,
        0x04,
        0x78,
        0x00,
        0x00, // m
        0x7C,
        0x08,
        0x04,
        0x04,
        0x78,
        0x00,
        0x00, // n
        0x38,
        0x44,
        0x44,
        0x44,
        0x38,
        0x00,
        0x00, // o
        0x7C,
        0x14,
        0x14,
        0x14,
        0x08,
        0x00,
        0x00, // p
        0x08,
        0x14,
        0x14,
        0x18,
        0x7C,
        0x00,
        0x00, // q
        0x7C,
        0x08,
        0x04,
        0x04,
        0x08,
        0x00,
        0x00, // r
        0x48,
        0x54,
        0x54,
        0x54,
        0x20,
        0x00,
        0x00, // s
        0x04,
        0x3F,
        0x44,
        0x40,
        0x20,
        0x00,
        0x00, // t
        0x3C,
        0x40,
        0x40,
        0x20,
        0x7C,
        0x00,
        0x00, // u
        0x1C,
        0x20,
        0x40,
        0x20,
        0x1C,
        0x00,
        0x00, // v
        0x3C,
        0x40,
        0x30,
        0x40,
        0x3C,
        0x00,
        0x00, // w
        0x00,
        0x44,
        0x28,
        0x10,
        0x28,
        0x44,
        0x00, // x
        0x0C,
        0x50,
        0x50,
        0x50,
        0x3C,
        0x00,
        0x00, // y
        0x44,
        0x64,
        0x54,
        0x4C,
        0x44,
        0x00,
        0x00, // z
        0x00,
        0x08,
        0x36,
        0x41,
        0x00,
        0x00,
        0x00, // {
        0x00,
        0x00,
        0x7F,
        0x00,
        0x00,
        0x00,
        0x00, // |
        0x00,
        0x41,
        0x36,
        0x08,
        0x00,
        0x00,
        0x00, // }
        0x08,
        0x08,
        0x2A,
        0x1C,
        0x08,
        0x00,
        0x00, // ->
        0x08,
        0x1C,
        0x2A,
        0x08,
        0x08,
        0x00,
        0x00, // <-
    ];

    for c in bytes {
        /* Create an array with our I2C instruction and a blank column at the end */
        let mut data: [u8; 9] = [SSD1306_BYTE_DATA, 0, 0, 0, 0, 0, 0, 0, 0];

        /* Calculate our index into the character table above */
        let index = (*c as usize - 0x20) * 7;

        /* Populate the middle of the array with the data from the character array at the right
         * index */
        data[1..8].copy_from_slice(&FONT_7X7[index..index + 7]);

        /* Write it out to the I2C bus */
        i2c.write(0x3C, &data)?
    }

    Ok(())
}

/// Initialise display with some useful values
fn ssd1306_init<I2C, E>(i2c: &mut I2C) -> Result<(), E>
where
    I2C: Write<Error = E>,
{
    i2c.write(0x3C, &[SSD1306_BYTE_CMD_SINGLE, SSD1306_DISPLAY_OFF])?;
    i2c.write(
        0x3C,
        &[SSD1306_BYTE_CMD_SINGLE, SSD1306_DISPLAY_CLK_DIV, 0x80],
    )?;
    i2c.write(0x3C, &[SSD1306_BYTE_CMD_SINGLE, SSD1306_SCAN_MODE_NORMAL])?;
    i2c.write(
        0x3C,
        &[SSD1306_BYTE_CMD_SINGLE, SSD1306_DISPLAY_OFFSET, 0x00, 0x00],
    )?;
    i2c.write(
        0x3C,
        &[SSD1306_BYTE_CMD_SINGLE, SSD1306_MEMORY_ADDR_MODE, 0x00],
    )?;
    i2c.write(
        0x3C,
        &[SSD1306_BYTE_CMD_SINGLE, SSD1306_DISPLAY_START_LINE, 0x00],
    )?;
    i2c.write(0x3C, &[SSD1306_BYTE_CMD_SINGLE, SSD1306_CHARGE_PUMP, 0x14])?;
    i2c.write(0x3C, &[SSD1306_BYTE_CMD_SINGLE, SSD1306_PIN_MAP, 0x12])?;
    i2c.write(0x3C, &[SSD1306_BYTE_CMD_SINGLE, SSD1306_DISPLAY_RAM])?;
    i2c.write(0x3C, &[SSD1306_BYTE_CMD_SINGLE, SSD1306_DISPLAY_NORMAL])?;
    i2c.write(0x3C, &[SSD1306_BYTE_CMD_SINGLE, SSD1306_DISPLAY_ON])?;

    let data = [
        SSD1306_BYTE_DATA,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
    ];

    for _ in 0..128 {
        i2c.write(0x3C, &data)?;
    }

    Ok(())
}

/// Position cursor at specified x, y block coordinate (multiple of 8)
fn ssd1306_pos<I2C, E>(i2c: &mut I2C, x: u8, y: u8) -> Result<(), E>
where
    I2C: Write<Error = E>,
{
    let data = [
        SSD1306_BYTE_CMD,
        SSD1306_COLUMN_RANGE,
        x * 8,
        0x7F,
        SSD1306_PAGE_RANGE,
        y,
        0x07,
    ];
    i2c.write(0x3C, &data)
}
