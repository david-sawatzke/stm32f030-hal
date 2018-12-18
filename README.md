stm32f030-hal
=============

## DEPRECATED
Use [stm32f0xx-hal][] instead.

_stm32f030-hal_ contains a hardware abstraction on top of the peripheral access
API for the STMicro stm32f030 series microcontroller. It's heavily based on
[stm32f042-hal][] by therealprof.

This crate implements a partial set of the [embedded-hal][] traits.

The following chips are supported, choose via features:
- stm32f030f4
- stm32f030k6
- stm32f030c6
- stm32f030c8
- stm32f030r8
- stm32f030cc
- stm32f030rc

Some of the implementation was shamelessly adapted from the [stm32f103xx-hal][]
crate by Jorge Aparicio.

[stm32f042-hal]: https://github.com/therealprof/stm32f042-hal.git
[stm32f103xx-hal]: https://github.com/japaric/stm32f103xx-hal
[embedded-hal]: https://github.com/japaric/embedded-hal.git
[nucleo-f042k6]: https://os.mbed.com/platforms/ST-Nucleo-F042K6/
[stm32f0xx-hal]: https://github.com/stm32-rs/stm32f0xx-hal

License
-------

[0-clause BSD license](LICENSE-0BSD.txt).
