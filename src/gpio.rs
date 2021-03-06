//! General Purpose Input / Output

use core::marker::PhantomData;

/// Extension trait to split a GPIO peripheral in independent pins and registers
pub trait GpioExt {
    /// The parts to split the GPIO into
    type Parts;

    /// Splits the GPIO block into independent pins and registers
    fn split(self) -> Self::Parts;
}

pub struct AF0;
pub struct AF1;
pub struct AF2;
pub struct AF3;
pub struct AF4;
pub struct AF5;
pub struct AF6;
pub struct AF7;

pub struct Alternate<MODE> {
    _mode: PhantomData<MODE>,
}

/// Input mode (type state)
pub struct Input<MODE> {
    _mode: PhantomData<MODE>,
}

/// Floating input (type state)
pub struct Floating;

/// Pulled down input (type state)
pub struct PullDown;

/// Pulled up input (type state)
pub struct PullUp;

/// Open drain input or output (type state)
pub struct OpenDrain;

/// Output mode (type state)
pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}

/// Push pull output (type state)
pub struct PushPull;

use hal::digital::{InputPin, OutputPin, StatefulOutputPin};
use stm32;

/// Fully erased pin
// We can just pretend it's gpioa. It's modified using the bits and it can only be constructed out of already existing pins
pub struct Pin<MODE> {
    i: u8,
    port: *const stm32::gpioa::RegisterBlock,
    _mode: PhantomData<MODE>,
}

impl<MODE> StatefulOutputPin for Pin<Output<MODE>> {
    fn is_set_high(&self) -> bool {
        !self.is_set_low()
    }

    fn is_set_low(&self) -> bool {
        // NOTE(unsafe) atomic read with no side effects
        unsafe { (*self.port).odr.read().bits() & (1 << self.i) == 0 }
    }
}

impl<MODE> OutputPin for Pin<Output<MODE>> {
    fn set_high(&mut self) {
        // NOTE(unsafe) atomic write to a stateless register
        unsafe { (*self.port).bsrr.write(|w| w.bits(1 << self.i)) }
    }

    fn set_low(&mut self) {
        // NOTE(unsafe) atomic write to a stateless register
        unsafe { (*self.port).bsrr.write(|w| w.bits(1 << (self.i + 16))) }
    }
}

impl InputPin for Pin<Output<OpenDrain>> {
    fn is_high(&self) -> bool {
        !self.is_low()
    }

    fn is_low(&self) -> bool {
        // NOTE(unsafe) atomic read with no side effects
        unsafe { (*self.port).idr.read().bits() & (1 << self.i) == 0 }
    }
}

impl<MODE> InputPin for Pin<Input<MODE>> {
    fn is_high(&self) -> bool {
        !self.is_low()
    }

    fn is_low(&self) -> bool {
        // NOTE(unsafe) atomic read with no side effects
        unsafe { (*self.port).idr.read().bits() & (1 << self.i) == 0 }
    }
}

macro_rules! gpio {
    ($GPIOX:ident, $gpiox:ident, $iopxenr:ident, $PXx:ident, [
        $($PXi:ident: ($pxi:ident, $i:expr, $MODE:ty),)+
    ]) => {
        /// GPIO
        pub mod $gpiox {
            use core::marker::PhantomData;

            use hal::digital::{InputPin, OutputPin, StatefulOutputPin};
            use stm32::$GPIOX;

            use stm32::RCC;
            use super::{
                Alternate, Floating, GpioExt, Input, OpenDrain, Output,
                PullDown, PullUp, PushPull, AF0, AF1, AF2, AF3, AF4, AF5, AF6, AF7, Pin
            };

            /// GPIO parts
            pub struct Parts {
                $(
                    /// Pin
                    pub $pxi: $PXi<$MODE>,
                )+
            }

            impl GpioExt for $GPIOX {
                type Parts = Parts;

                fn split(self) -> Parts {
                    // NOTE(unsafe) This executes only during initialisation
                    let rcc = unsafe { &(*RCC::ptr()) };
                    rcc.ahbenr.modify(|_, w| w.$iopxenr().set_bit());

                    Parts {
                        $(
                            $pxi: $PXi { _mode: PhantomData },
                        )+
                    }
                }
            }

            /// Partially erased pin
            pub struct $PXx<MODE> {
                i: u8,
                _mode: PhantomData<MODE>,
            }

            impl<MODE> StatefulOutputPin for $PXx<Output<MODE>> {
                fn is_set_high(&self) -> bool {
                    !self.is_set_low()
                }

                fn is_set_low(&self) -> bool {
                    // NOTE(unsafe) atomic read with no side effects
                    unsafe { (*$GPIOX::ptr()).odr.read().bits() & (1 << self.i) == 0 }
                }
            }

            impl<MODE> OutputPin for $PXx<Output<MODE>> {
                fn set_high(&mut self) {
                    // NOTE(unsafe) atomic write to a stateless register
                    unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << self.i)) }
                }

                fn set_low(&mut self) {
                    // NOTE(unsafe) atomic write to a stateless register
                    unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << (self.i + 16))) }
                }
            }

            impl InputPin for $PXx<Output<OpenDrain>> {
                fn is_high(&self) -> bool {
                    !self.is_low()
                }

                fn is_low(&self) -> bool {
                    // NOTE(unsafe) atomic read with no side effects
                    unsafe { (*$GPIOX::ptr()).idr.read().bits() & (1 << self.i) == 0 }
                }
            }

            impl<MODE> InputPin for $PXx<Input<MODE>> {
                fn is_high(&self) -> bool {
                    !self.is_low()
                }

                fn is_low(&self) -> bool {
                    // NOTE(unsafe) atomic read with no side effects
                    unsafe { (*$GPIOX::ptr()).idr.read().bits() & (1 << self.i) == 0 }
                }
            }

            fn _set_alternate_mode (index:usize, mode: u32)
            {
                let offset = 2 * index;
                let offset2 = 4 * index;
                unsafe {
                    if offset2 < 32 {
                        &(*$GPIOX::ptr()).afrl.modify(|r, w| {
                            w.bits((r.bits() & !(0b1111 << offset2)) | (mode << offset2))
                        });
                    } else
                    {
                        let offset2 = offset2 - 32;
                        &(*$GPIOX::ptr()).afrh.modify(|r, w| {
                            w.bits((r.bits() & !(0b1111 << offset2)) | (mode << offset2))
                        });
                    }
                    &(*$GPIOX::ptr()).moder.modify(|r, w| {
                        w.bits((r.bits() & !(0b11 << offset)) | (0b10 << offset))
                    });
                }
            }

            impl<MODE> $PXx<Input<MODE>> {
                /// Erases the port from the type
                ///
                /// This is useful when you want to collect the pins into an array where you
                /// need all the elements to have the same type
                pub fn downgrade(self) -> Pin<Input<MODE>> {
                    use stm32::gpioa;
                    use core::intrinsics::transmute;
                    Pin {
                        i: self.i,
                        port: unsafe{ transmute::<_, *const gpioa::RegisterBlock>($GPIOX::ptr())},
                        _mode: self._mode,
                    }
                }
            }

            impl<MODE> $PXx<Output<MODE>> {
                /// Erases the port from the type
                ///
                /// This is useful when you want to collect the pins into an array where you
                /// need all the elements to have the same type
                pub fn downgrade(self) -> Pin<Output<MODE>> {
                    use stm32::gpioa;
                    use core::intrinsics::transmute;
                    Pin {
                        i: self.i,
                        port: unsafe{ transmute::<_, *const gpioa::RegisterBlock>($GPIOX::ptr())},
                        _mode: self._mode,
                    }
                }
            }

            $(
                /// Pin
                pub struct $PXi<MODE> {
                    _mode: PhantomData<MODE>,
                }

                impl<MODE> $PXi<MODE> {
                    /// Configures the pin to operate in AF0 mode
                    pub fn into_alternate_af0(
                        self,
                    ) -> $PXi<Alternate<AF0>> {
                        _set_alternate_mode($i, 0);
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate in AF1 mode
                    pub fn into_alternate_af1(
                        self,
                    ) -> $PXi<Alternate<AF1>> {
                        _set_alternate_mode($i, 1);
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate in AF2 mode
                    pub fn into_alternate_af2(
                        self,
                    ) -> $PXi<Alternate<AF2>> {
                        _set_alternate_mode($i, 2);
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate in AF3 mode
                    pub fn into_alternate_af3(
                        self,
                    ) -> $PXi<Alternate<AF3>> {
                        _set_alternate_mode($i, 3);
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate in AF4 mode
                    pub fn into_alternate_af4(
                        self,
                    ) -> $PXi<Alternate<AF4>> {
                        _set_alternate_mode($i, 4);
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate in AF5 mode
                    pub fn into_alternate_af5(
                        self,
                    ) -> $PXi<Alternate<AF5>> {
                        _set_alternate_mode($i, 5);
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate in AF6 mode
                    pub fn into_alternate_af6(
                        self,
                    ) -> $PXi<Alternate<AF6>> {
                        _set_alternate_mode($i, 6);
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate in AF7 mode
                    pub fn into_alternate_af7(
                        self,
                    ) -> $PXi<Alternate<AF7>> {
                        _set_alternate_mode($i, 7);
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as a floating input pin
                    pub fn into_floating_input(
                        self,
                    ) -> $PXi<Input<Floating>> {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                        }
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as a pulled down input pin
                    pub fn into_pull_down_input(
                        self,
                    ) -> $PXi<Input<PullDown>> {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b10 << offset))
                            });
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                        }
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as a pulled up input pin
                    pub fn into_pull_up_input(
                        self,
                    ) -> $PXi<Input<PullUp>> {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                            });
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                        }
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as an open drain output pin
                    pub fn into_open_drain_output(
                        self,
                    ) -> $PXi<Output<OpenDrain>> {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                            &(*$GPIOX::ptr()).otyper.modify(|r, w| {
                                w.bits(r.bits() | (0b1 << $i))
                            });
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                            });
                        }
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as an push pull output pin
                    pub fn into_push_pull_output(
                        self,
                    ) -> $PXi<Output<PushPull>> {
                        let offset = 2 * $i;

                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                            &(*$GPIOX::ptr()).otyper.modify(|r, w| {
                                w.bits(r.bits() & !(0b1 << $i))
                            });
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                            });
                        }
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as an push pull output pin with quick fall
                    /// and rise times
                    pub fn into_push_pull_output_hs(
                        self,
                    ) -> $PXi<Output<PushPull>> {
                        let offset = 2 * $i;

                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                            &(*$GPIOX::ptr()).otyper.modify(|r, w| {
                                w.bits(r.bits() & !(0b1 << $i))
                            });
                            &(*$GPIOX::ptr()).ospeedr.modify(|r, w| {
                                w.bits(r.bits() & !(0b1 << $i))
                            });
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                            });
                        }

                        $PXi { _mode: PhantomData }
                    }
                }

                impl $PXi<Output<OpenDrain>> {
                    /// Enables / disables the internal pull up
                    pub fn internal_pull_up(&mut self, on: bool) {
                        let offset = 2 * $i;
                        let value = if on { 0b01 } else { 0b00 };
                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (value << offset))
                            })};
                    }
                }

                impl InputPin for $PXi<Output<OpenDrain>> {
                    fn is_high(&self) -> bool {
                        !self.is_low()
                    }

                    fn is_low(&self) -> bool {
                        // NOTE(unsafe) atomic read with no side effects
                        unsafe { (*$GPIOX::ptr()).idr.read().bits() & (1 << $i) == 0 }
                    }
                }

                impl<MODE> $PXi<Alternate<MODE>> {
                    /// Enables / disables the internal pull up
                    pub fn internal_pull_up(&mut self, on: bool) {
                        let offset = 2 * $i;
                        let value = if on { 0b01 } else { 0b00 };
                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (value << offset))
                            })};
                    }
                }

                impl<MODE> $PXi<Alternate<MODE>> {
                    /// Turns pin alternate configuration pin into open drain
                    pub fn set_open_drain(self) -> Self {
                        let offset = $i;
                        unsafe {
                            &(*$GPIOX::ptr()).otyper.modify(|r, w| {
                                w.bits(r.bits() | (1 << offset))
                            })};

                        self
                    }
                }

                impl<MODE> $PXi<Output<MODE>> {
                    /// Erases the pin number from the type
                    ///
                    /// This is useful when you want to collect the pins into an array where you
                    /// need all the elements to have the same type
                    pub fn downgrade(self) -> $PXx<Output<MODE>> {
                        $PXx {
                            i: $i,
                            _mode: self._mode,
                        }
                    }
                }

                impl<MODE> StatefulOutputPin for $PXi<Output<MODE>> {
                    fn is_set_high(&self) -> bool {
                        !self.is_set_low()
                    }

                    fn is_set_low(&self) -> bool {
                        // NOTE(unsafe) atomic read with no side effects
                        unsafe { (*$GPIOX::ptr()).odr.read().bits() & (1 << $i) == 0 }
                    }
                }

                impl<MODE> OutputPin for $PXi<Output<MODE>> {
                    fn set_high(&mut self) {
                        // NOTE(unsafe) atomic write to a stateless register
                        unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << $i)) }
                    }

                    fn set_low(&mut self) {
                        // NOTE(unsafe) atomic write to a stateless register
                        unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << ($i + 16))) }
                    }
                }

                impl<MODE> $PXi<Input<MODE>> {
                    /// Erases the pin number from the type
                    ///
                    /// This is useful when you want to collect the pins into an array where you
                    /// need all the elements to have the same type
                    pub fn downgrade(self) -> $PXx<Input<MODE>> {
                        $PXx {
                            i: $i,
                            _mode: self._mode,
                        }
                    }
                }

                impl<MODE> InputPin for $PXi<Input<MODE>> {
                    fn is_high(&self) -> bool {
                        !self.is_low()
                    }

                    fn is_low(&self) -> bool {
                        // NOTE(unsafe) atomic read with no side effects
                        unsafe { (*$GPIOX::ptr()).idr.read().bits() & (1 << $i) == 0 }
                    }
                }
            )+
            /// Get the pin number
            impl<TYPE> $PXx<TYPE> {
                pub fn get_id (&self) -> u8
                {
                    self.i
                }
            }
        }
    }
}

// TSSOP20
#[cfg(any(feature = "stm32f030f4"))]
gpio!(GPIOA, gpioa, iopaen, PA, [
    PA0: (pa0, 0, Input<Floating>),
    PA1: (pa1, 1, Input<Floating>),
    PA2: (pa2, 2, Input<Floating>),
    PA3: (pa3, 3, Input<Floating>),
    PA4: (pa4, 4, Input<Floating>),
    PA5: (pa5, 5, Input<Floating>),
    PA6: (pa6, 6, Input<Floating>),
    PA7: (pa7, 7, Input<Floating>),
    PA9: (pa9, 9, Input<Floating>),
    PA10: (pa10, 10, Input<Floating>),
    PA13: (pa13, 13, Input<Floating>),
    PA14: (pa14, 14, Input<Floating>),
]);

// LQFP32 & LQFP48 & LQFP64
#[cfg(any(
    feature = "stm32f030k6",
    feature = "stm32f030c6",
    feature = "stm32f030c8",
    feature = "stm32f030cc",
    feature = "stm32f030r8",
    feature = "stm32f030rc"
))]
gpio!(GPIOA, gpioa, iopaen, PA, [
    PA0: (pa0, 0, Input<Floating>),
    PA1: (pa1, 1, Input<Floating>),
    PA2: (pa2, 2, Input<Floating>),
    PA3: (pa3, 3, Input<Floating>),
    PA4: (pa4, 4, Input<Floating>),
    PA5: (pa5, 5, Input<Floating>),
    PA6: (pa6, 6, Input<Floating>),
    PA7: (pa7, 7, Input<Floating>),
    PA8: (pa8, 8, Input<Floating>),
    PA9: (pa9, 9, Input<Floating>),
    PA10: (pa10, 10, Input<Floating>),
    PA11: (pa11, 11, Input<Floating>),
    PA12: (pa12, 12, Input<Floating>),
    PA13: (pa13, 13, Input<Floating>),
    PA14: (pa14, 14, Input<Floating>),
    PA15: (pa15, 15, Input<Floating>),
]);

// TSSOP20
#[cfg(any(feature = "stm32f030x4"))]
gpio!(GPIOB, gpiob, iopben, PB, [
    PB1: (pb1, 1, Input<Floating>),
]);

// LQFP32
#[cfg(any(feature = "stm32f030k6"))]
gpio!(GPIOB, gpiob, iopben, PB, [
    PB0: (pb0, 0, Input<Floating>),
    PB1: (pb1, 1, Input<Floating>),
    PB3: (pb3, 3, Input<Floating>),
    PB4: (pb4, 4, Input<Floating>),
    PB5: (pb5, 5, Input<Floating>),
    PB6: (pb6, 6, Input<Floating>),
    PB7: (pb7, 7, Input<Floating>),
]);

// LQFP48 & LQFP64
#[cfg(any(
    feature = "stm32f030c6",
    feature = "stm32f030c8",
    feature = "stm32f030cc",
    feature = "stm32f030r8",
    feature = "stm32f030rc"
))]
gpio!(GPIOB, gpiob, iopben, PB, [
    PB0: (pb0, 0, Input<Floating>),
    PB1: (pb1, 1, Input<Floating>),
    PB2: (pb2, 2, Input<Floating>),
    PB3: (pb3, 3, Input<Floating>),
    PB4: (pb4, 4, Input<Floating>),
    PB5: (pb5, 5, Input<Floating>),
    PB6: (pb6, 6, Input<Floating>),
    PB7: (pb7, 7, Input<Floating>),
    PB8: (pb8, 8, Input<Floating>),
    PB9: (pb9, 9, Input<Floating>),
    PB10: (pb10, 10, Input<Floating>),
    PB11: (pb11, 11, Input<Floating>),
    PB12: (pb12, 12, Input<Floating>),
    PB13: (pb13, 13, Input<Floating>),
    PB14: (pb14, 14, Input<Floating>),
    PB15: (pb15, 15, Input<Floating>),
]);

// LQFP48
#[cfg(any(
    feature = "stm32f030c6",
    feature = "stm32f030c8",
    feature = "stm32f030cc",
))]
gpio!(GPIOC, gpioc, iopcen, PC, [
    PC13: (pb13, 13, Input<Floating>),
    PC14: (pb14, 14, Input<Floating>),
    PC15: (pb15, 15, Input<Floating>),
]);

// LQFP64
#[cfg(any(feature = "stm32f030r8", feature = "stm32f030rc"))]
gpio!(GPIOC, gpioc, iopcen, PC, [
    PC0: (pb0, 0, Input<Floating>),
    PC1: (pb1, 1, Input<Floating>),
    PC2: (pb2, 2, Input<Floating>),
    PC3: (pb3, 3, Input<Floating>),
    PC4: (pb4, 4, Input<Floating>),
    PC5: (pb5, 5, Input<Floating>),
    PC6: (pb6, 6, Input<Floating>),
    PC7: (pb7, 7, Input<Floating>),
    PC8: (pb8, 8, Input<Floating>),
    PC9: (pb9, 9, Input<Floating>),
    PC10: (pb10, 10, Input<Floating>),
    PC11: (pb11, 11, Input<Floating>),
    PC12: (pb12, 12, Input<Floating>),
    PC13: (pb13, 13, Input<Floating>),
    PC14: (pb14, 14, Input<Floating>),
    PC15: (pb15, 15, Input<Floating>),
]);

// TODO Check if the bit is implemented
// In the device crate the iopden bit is missing, so it won't compile
// // LQFP64
// #[cfg(any(feature = "stm32f030r8", feature = "stm32f030rc"))]
// gpio!(GPIOD, gpiod, iopden, PD, [
//     PD2: (pd2, 2, Input<Floating>),
// ]);

// TSSOP20 & LQFP32
#[cfg(any(
    feature = "stm32f030f4",
    feature = "stm32f030k6",
    feature = "stm32f030cc",
    feature = "stm32f030rc"
))]
gpio!(GPIOF, gpiof, iopfen, PF, [
    PF0: (pf0, 0, Input<Floating>),
    PF1: (pf1, 1, Input<Floating>),
]);

// LQFP48
#[cfg(any(feature = "stm32f030c6", feature = "stm32f030c8",))]
gpio!(GPIOF, gpiof, iopfen, PF, [
    PF0: (pf0, 0, Input<Floating>),
    PF1: (pf1, 1, Input<Floating>),
    // STM32F030x4/6/8
    PF6: (pf5, 5, Input<Floating>),
    PF7: (pf5, 5, Input<Floating>),
]);

// LQFP64
// LQFP64
#[cfg(feature = "stm32f030r8")]
gpio!(GPIOF, gpiof, iopfen, PF, [
    PF0: (pf0, 0, Input<Floating>),
    PF1: (pf1, 1, Input<Floating>),
    // STM32F030x4/6/8
    PF4: (pf4, 4, Input<Floating>),
    PF5: (pf5, 5, Input<Floating>),
    PF6: (pf5, 5, Input<Floating>),
    PF7: (pf5, 5, Input<Floating>),
]);
