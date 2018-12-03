use core::fmt::{Result, Write};
use core::marker::PhantomData;
use core::ptr;

use hal;
use hal::prelude::*;
use nb;
use void::Void;

use stm32::{RCC, USART1};

use gpio::gpioa::{PA10, PA2, PA3, PA9};
use gpio::{Alternate, AF1};
use rcc::Clocks;
use time::Bps;

/// Interrupt event
pub enum Event {
    /// New data has been received
    Rxne,
    /// New data can be sent
    Txe,
}

/// Serial error
#[derive(Debug)]
pub enum Error {
    /// Framing error
    Framing,
    /// Noise error
    Noise,
    /// RX buffer overrun
    Overrun,
    /// Parity check error
    Parity,
    #[doc(hidden)]
    _Extensible,
}

pub trait Pins<USART> {}

impl Pins<USART1> for (PA9<Alternate<AF1>>, PA10<Alternate<AF1>>) {}
impl Pins<USART1> for (PA2<Alternate<AF1>>, PA3<Alternate<AF1>>) {}
impl Pins<USART1> for (PA9<Alternate<AF1>>, PA3<Alternate<AF1>>) {}
impl Pins<USART1> for (PA2<Alternate<AF1>>, PA10<Alternate<AF1>>) {}

/// Serial abstraction
pub struct Serial<USART, PINS> {
    usart: USART,
    pins: PINS,
}

/// Serial receiver
pub struct Rx<USART> {
    _usart: PhantomData<USART>,
}

/// Serial transmitter
pub struct Tx<USART> {
    _usart: PhantomData<USART>,
}

/// USART1
impl<PINS> Serial<USART1, PINS> {
    pub fn usart1(usart: USART1, pins: PINS, baud_rate: Bps, clocks: Clocks) -> Self
    where
        PINS: Pins<USART1>,
    {
        // NOTE(unsafe) This executes only during initialisation
        let rcc = unsafe { &(*RCC::ptr()) };

        /* Enable clock for USART */
        rcc.apb2enr.modify(|_, w| w.usart1en().set_bit());

        // Calculate correct baudrate divisor on the fly
        let brr = clocks.pclk().0 / baud_rate.0;
        usart.brr.write(|w| unsafe { w.bits(brr) });

        /* Reset other registers to disable advanced USART features */
        usart.cr2.reset();
        usart.cr3.reset();

        /* Enable transmission and receiving */
        usart.cr1.modify(|_, w| unsafe { w.bits(0xD) });

        Serial { usart, pins }
    }

    pub fn split(self) -> (Tx<USART1>, Rx<USART1>) {
        (
            Tx {
                _usart: PhantomData,
            },
            Rx {
                _usart: PhantomData,
            },
        )
    }
    pub fn release(self) -> (USART1, PINS) {
        (self.usart, self.pins)
    }
}

impl hal::serial::Read<u8> for Rx<USART1> {
    type Error = Error;

    fn read(&mut self) -> nb::Result<u8, Error> {
        // NOTE(unsafe) atomic read with no side effects
        let isr = unsafe { (*USART1::ptr()).isr.read() };

        Err(if isr.pe().bit_is_set() {
            nb::Error::Other(Error::Parity)
        } else if isr.fe().bit_is_set() {
            nb::Error::Other(Error::Framing)
        } else if isr.nf().bit_is_set() {
            nb::Error::Other(Error::Noise)
        } else if isr.ore().bit_is_set() {
            nb::Error::Other(Error::Overrun)
        } else if isr.rxne().bit_is_set() {
            // NOTE(read_volatile) see `write_volatile` below
            return Ok(unsafe { ptr::read_volatile(&(*USART1::ptr()).rdr as *const _ as *const _) });
        } else {
            nb::Error::WouldBlock
        })
    }
}

impl hal::serial::Write<u8> for Tx<USART1> {
    type Error = Void;

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        // NOTE(unsafe) atomic read with no side effects
        let isr = unsafe { (*USART1::ptr()).isr.read() };

        if isr.tc().bit_is_set() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
        // NOTE(unsafe) atomic read with no side effects
        let isr = unsafe { (*USART1::ptr()).isr.read() };

        if isr.txe().bit_is_set() {
            // NOTE(unsafe) atomic write to stateless register
            // NOTE(write_volatile) 8-bit write that's not possible through the svd2rust API
            unsafe { ptr::write_volatile(&(*USART1::ptr()).tdr as *const _ as *mut _, byte) }
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl<USART> Write for Tx<USART>
where
    Tx<USART>: hal::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> Result {
        let _ = s.as_bytes().iter().map(|c| block!(self.write(*c))).last();
        Ok(())
    }
}
