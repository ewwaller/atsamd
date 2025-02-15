#![no_std]

extern crate atsamd_hal as hal;

#[cfg(feature = "rt")]
extern crate cortex_m_rt;
#[cfg(feature = "rt")]
pub use cortex_m_rt::entry;

use embedded_hal;
use hal::*;

pub use hal::common::*;

pub use hal::target_device as pac;

use gpio::v2::AnyPin;
use gpio::Port;
use hal::clock::GenericClockController;
use hal::sercom::v2::spi;
use hal::sercom::{I2CMaster0, PadPin, UART5};
use hal::time::Hertz;

#[cfg(feature = "usb")]
use hal::usb::usb_device::bus::UsbBusAllocator;
#[cfg(feature = "usb")]
pub use hal::usb::UsbBus;

bsp_pins!(
    PA02 {
        name: d15,
        aliases: {
            AlternateB: A0
        }
    }
    PA04 {
        name: d18,
        aliases: {
            AlternateB: A3,
            PushPullOutput: BaseSlaveSelect
        }
    }
    PA05 {
        name: d19,
        aliases: {
            AlternateB: A4,
            PullUpInput: BaseSlaveAck
        }
    }
    PA06 {
        name: d20,
        aliases: {
            AlternateB: A5
        }
    }
    PA07 {
        name: d21,
        aliases: {
            AlternateB: A6,
            AlternateG: I2sSerialData
        }
    }
    PA08 {
        name: d11,
        aliases: {
            AlternateC: Sda
        }
    }
    PA09 {
        name: d12,
        aliases: {
            AlternateC: Scl
        }
    }
    PA10 {
        name: d2,
        aliases: {
            AlternateG: I2sSerialClock
        }
    }
    PA11 { name: d3 }
    PA12 {
        name: pa12,
        aliases: {
            AlternateC: SdMosi
        }
    }
    PA13 {
        name: pa13,
        aliases: {
            AlternateC: SdSck
        }
    }
    PA14 {
        name: pa14,
        aliases: {
            PushPullOutput: SdSlaveSelect
        }
    }
    PA15 {
        name: pa15,
        aliases: {
            AlternateC: SdMiso
        }
    }
    PA16 {
        name: d8,
        aliases: {
            AlternateC: Spi0Mosi
        }
    }
    PA17 {
        name: d9,
        aliases: {
            AlternateC: Spi0Sck
        }
    }
    PA18 {
        name: pa18,
        aliases: {
            /// Host Enable, drive high to switch into USB host mode
            #[cfg(feature = "usb")]
            PushPullOutput: UsbId
        }
    }
    PA19 {
        name: d10,
        aliases: {
            AlternateC: Spi0Miso
        }
    }
    PA20 { name: d6 }
    PA21 { name: d7 }
    PA22 { name: d0 }
    PA23 { name: d1 }
    PA24 {
        name: usb_dm,
        aliases: {
            #[cfg(feature = "usb")]
            AlternateG: UsbDm
        }
    }
    PA25 {
        name: usb_dp,
        aliases: {
            #[cfg(feature = "usb")]
            AlternateG: UsbDp
        }
    }
    PA27 {
        name: pa27,
        aliases: {
            PullUpInput: SdCardDetect
        }
    }
    PA28 {
        name: switch,
        aliases: {
            PullUpInput: Switch
        }
    }
    PB02 {
        name: d16,
        aliases: {
            AlternateB: A1
        }
    }
    PB03 {
        name: d17,
        aliases: {
            AlternateB: A2
        }
    }
    PB08 {
        name: led,
        aliases: {
            PushPullOutput: Led
        }
    }
    PB09 {
        name: pb09,
        aliases: {
            PushPullOutput: BaseEnable,
            AlternateB: AdcBattery
        }
    }
    PB10 { name: d4 }
    PB11 { name: d5 }
    PB22 {
        name: d14,
        aliases: {
            AlternateD: UartTx
        }
    }
    PB23 {
        name: d13,
        aliases: {
            AlternateD: UartRx
        }
    }
);

const BASE_CONTROLLER_FREQ: Hertz = Hertz(1000000);
const BASE_CONTROLLER_SPI_MODE: embedded_hal::spi::Mode = spi::MODE_2;

/// Convenience for setting up the labeled SPI0 peripheral.
/// SPI0 has the P1AM base controller connected.
/// This powers up SERCOM1 and configures it for talking to the
/// base controller.
pub fn base_controller_spi(
    clocks: &mut GenericClockController,
    sercom1: pac::SERCOM1,
    pm: &mut pac::PM,
    sck: Spi0Sck,
    mosi: Spi0Mosi,
    miso: Spi0Miso,
) -> sercom::v2::spi::Spi<impl spi::ValidConfig> {
    let gclk0 = &clocks.gclk0();
    let core_clock = &clocks.sercom1_core(&gclk0).unwrap();
    let pads = spi::Pads::new().sclk(sck).data_in(miso).data_out(mosi);
    spi::Config::new(pm, sercom1, pads, core_clock.freq())
        .baud(BASE_CONTROLLER_FREQ)
        .spi_mode(BASE_CONTROLLER_SPI_MODE)
        .enable()
}

/// Convenience for setting up the labeled SPI2 peripheral.
/// SPI2 has the microSD card slot connected.
/// This powers up SERCOM4 and configures it for talking to the
/// base controller.
pub fn sdmmc_spi<F: Into<Hertz>>(
    clocks: &mut GenericClockController,
    bus_speed: F,
    sercom4: pac::SERCOM4,
    pm: &mut pac::PM,
    sck: SdSck,
    mosi: SdMosi,
    miso: SdMiso,
) -> sercom::v2::spi::Spi<impl spi::ValidConfig> {
    let gclk0 = &clocks.gclk0();
    let core_clock = &clocks.sercom4_core(&gclk0).unwrap();
    let pads = spi::Pads::new().sclk(sck).data_in(miso).data_out(mosi);
    spi::Config::new(pm, sercom4, pads, core_clock.freq())
        .baud(bus_speed)
        .enable()
}

/// Convenience for setting up the labelled SDA, SCL pins to
/// operate as an I2C master running at the specified frequency.
pub fn i2c_master<F: Into<Hertz>>(
    clocks: &mut GenericClockController,
    bus_speed: F,
    sercom0: pac::SERCOM0,
    pm: &mut pac::PM,
    sda: Sda,
    scl: Scl,
    port: &mut Port,
) -> I2CMaster0<
    impl hal::sercom::AnyPad<Sercom = pac::SERCOM0>,
    impl hal::sercom::AnyPad<Sercom = pac::SERCOM0>,
> {
    let gclk0 = clocks.gclk0();
    I2CMaster0::new(
        &clocks.sercom0_core(&gclk0).unwrap(),
        bus_speed.into(),
        sercom0,
        pm,
        <hal::gpio::v1::Pin<_, _>>::from(sda).into_pad(port),
        <hal::gpio::v1::Pin<_, _>>::from(scl).into_pad(port),
    )
}

pub type Uart = UART5<
    hal::sercom::Sercom5Pad3<<UartRx as AnyPin>::Id>,
    hal::sercom::Sercom5Pad2<<UartTx as AnyPin>::Id>,
    (),
    (),
>;

/// Convenience for setting up the labelled RX, TX pins to
/// operate as a UART device running at the specified baud.
pub fn uart<F: Into<Hertz>>(
    clocks: &mut GenericClockController,
    baud: F,
    sercom5: pac::SERCOM5,
    pm: &mut pac::PM,
    rx: UartRx,
    tx: UartTx,
) -> Uart {
    let gclk0 = clocks.gclk0();

    UART5::new(
        &clocks.sercom5_core(&gclk0).unwrap(),
        baud.into(),
        sercom5,
        pm,
        (rx.into(), tx.into()),
    )
}

#[cfg(feature = "usb")]
pub fn usb_allocator(
    usb: pac::USB,
    clocks: &mut GenericClockController,
    pm: &mut pac::PM,
    dm: UsbDm,
    dp: UsbDp,
) -> UsbBusAllocator<UsbBus> {
    let gclk0 = clocks.gclk0();
    let usb_clock = &clocks.usb(&gclk0).unwrap();

    UsbBusAllocator::new(UsbBus::new(usb_clock, pm, dm, dp, usb))
}
