use core::fmt::{Result, Write};
use core::marker::PhantomData;
use core::ptr;

use hal;
use hal::prelude::*;
use nb;
use void::Void;

use stm32::{RCC, USART1, USART2, USART3, USART4, USART5, USART6};

use gpio::*;
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

// The pin combinations are missing. Only grouped pins are defined.
// TODO find a good way to do that automatically
#[cfg(any(
    feature = "stm32f030f4",
    feature = "stm32f030k6",
    feature = "stm32f030c6",
    feature = "stm32f030c8",
    feature = "stm32f030cc",
    feature = "stm32f030r8",
    feature = "stm32f030rc"
))]
impl Pins<USART1> for (gpioa::PA9<Alternate<AF1>>, gpioa::PA10<Alternate<AF1>>) {}
#[cfg(any(
    feature = "stm32f030f4",
    feature = "stm32f030k6",
    feature = "stm32f030c6",
))]
impl Pins<USART1> for (gpioa::PA2<Alternate<AF1>>, gpioa::PA3<Alternate<AF1>>) {}
#[cfg(any(feature = "stm32f030k6", feature = "stm32f030c6",))]
impl Pins<USART1> for (gpioa::PA14<Alternate<AF1>>, gpioa::PA15<Alternate<AF1>>) {}
#[cfg(any(
    feature = "stm32f030k6",
    feature = "stm32f030c6",
    feature = "stm32f030c8",
    feature = "stm32f030cc",
    feature = "stm32f030r8",
    feature = "stm32f030rc"
))]
impl Pins<USART1> for (gpiob::PB6<Alternate<AF0>>, gpiob::PB7<Alternate<AF0>>) {}
#[cfg(any(
    feature = "stm32f030c8",
    feature = "stm32f030cc",
    feature = "stm32f030r8",
    feature = "stm32f030rc"
))]
impl Pins<USART2> for (gpioa::PA2<Alternate<AF1>>, gpioa::PA3<Alternate<AF1>>) {}
#[cfg(any(
    feature = "stm32f030c8",
    feature = "stm32f030cc",
    feature = "stm32f030r8",
    feature = "stm32f030rc"
))]
impl Pins<USART2> for (gpioa::PA14<Alternate<AF1>>, gpioa::PA15<Alternate<AF1>>) {}
// TODO Proper mapping for all pins
#[cfg(any(feature = "stm32f030cc", feature = "stm32f030rc"))]
impl Pins<USART3> for (gpiob::PB10<Alternate<AF4>>, gpiob::PB11<Alternate<AF4>>) {}
#[cfg(any(feature = "stm32f030cc", feature = "stm32f030rc"))]
impl Pins<USART4> for (gpioa::PA0<Alternate<AF4>>, gpioa::PA1<Alternate<AF4>>) {}
#[cfg(any(feature = "stm32f030cc", feature = "stm32f030rc"))]
impl Pins<USART5> for (gpiob::PB3<Alternate<AF4>>, gpiob::PB4<Alternate<AF4>>) {}
#[cfg(any(feature = "stm32f030cc", feature = "stm32f030rc"))]
impl Pins<USART6> for (gpioa::PA4<Alternate<AF4>>, gpioa::PA5<Alternate<AF4>>) {}

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

macro_rules! usart {
    ($($USART:ident: ($usart:ident, $usartXen:ident, $apbenr:ident),)+) => {
        $(
            /// USART
            impl<PINS> Serial<$USART, PINS> {
                pub fn $usart(usart: $USART, pins: PINS, baud_rate: Bps, clocks: Clocks) -> Self
                where
                    PINS: Pins<$USART>,
                {
                    // NOTE(unsafe) This executes only during initialisation
                    let rcc = unsafe { &(*RCC::ptr()) };

                    /* Enable clock for USART */
                    rcc.$apbenr.modify(|_, w| w.$usartXen().set_bit());

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

                pub fn split(self) -> (Tx<$USART>, Rx<$USART>) {
                    (
                        Tx {
                            _usart: PhantomData,
                        },
                        Rx {
                            _usart: PhantomData,
                        },
                    )
                }
                pub fn release(self) -> ($USART, PINS) {
                    (self.usart, self.pins)
                }
            }

            impl hal::serial::Read<u8> for Rx<$USART> {
                type Error = Error;

                fn read(&mut self) -> nb::Result<u8, Error> {
                    // NOTE(unsafe) atomic read with no side effects
                    let isr = unsafe { (*$USART::ptr()).isr.read() };

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
                        return Ok(unsafe { ptr::read_volatile(&(*$USART::ptr()).rdr as *const _ as *const _) });
                    } else {
                        nb::Error::WouldBlock
                    })
                }
            }

            impl hal::serial::Write<u8> for Tx<$USART> {
                type Error = Void;

                fn flush(&mut self) -> nb::Result<(), Self::Error> {
                    // NOTE(unsafe) atomic read with no side effects
                    let isr = unsafe { (*$USART::ptr()).isr.read() };

                    if isr.tc().bit_is_set() {
                        Ok(())
                    } else {
                        Err(nb::Error::WouldBlock)
                    }
                }

                fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
                    // NOTE(unsafe) atomic read with no side effects
                    let isr = unsafe { (*$USART::ptr()).isr.read() };

                    if isr.txe().bit_is_set() {
                        // NOTE(unsafe) atomic write to stateless register
                        // NOTE(write_volatile) 8-bit write that's not possible through the svd2rust API
                        unsafe { ptr::write_volatile(&(*$USART::ptr()).tdr as *const _ as *mut _, byte) }
                        Ok(())
                    } else {
                        Err(nb::Error::WouldBlock)
                    }
                }
            }

        )+
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

#[cfg(any(
    feature = "stm32f030f4",
    feature = "stm32f030k6",
    feature = "stm32f030c6",
    feature = "stm32f030c8",
    feature = "stm32f030cc",
    feature = "stm32f030r8",
    feature = "stm32f030rc"
))]
usart! {
    USART1: (usart1, usart1en, apb2enr),
}

#[cfg(any(
    feature = "stm32f030c8",
    feature = "stm32f030cc",
    feature = "stm32f030r8",
    feature = "stm32f030rc"
))]
usart! {
    USART2: (usart2, usart2en, apb1enr),
}

#[cfg(any(feature = "stm32f030cc", feature = "stm32f030rc"))]
usart! {
    USART3: (usart3, usart3en, apb1enr),
    USART4: (usart4, usart4en, apb1enr),
    USART5: (usart5, usart5en, apb1enr),
    // the usart6en bit is missing
    // USART6: (usart6, usart6en, apb2enr),
}
