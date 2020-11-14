use stm32f1xx_hal as hal;

use hal::{
    afio,
    prelude::*,
    rcc,
    serial::{self, Serial},
    stm32,
    usb::{Peripheral, UsbBus},
};
pub use panel_protocol::{Command, CommandReader, Report};
use usb_device::{device::UsbDevice, UsbError};
use usbd_serial::SerialPort;

#[derive(Debug)]
pub enum Error {
    Serial(hal::serial::Error),
    UsbError(UsbError),
    BufferFull,
    MalformedMessage,
}

impl From<serial::Error> for Error {
    fn from(e: serial::Error) -> Error {
        Error::Serial(e)
    }
}

impl From<UsbError> for Error {
    fn from(e: UsbError) -> Error {
        Error::UsbError(e)
    }
}

impl From<panel_protocol::Error> for Error {
    fn from(e: panel_protocol::Error) -> Error {
        match e {
            panel_protocol::Error::BufferFull => Error::BufferFull,
            panel_protocol::Error::MalformedMessage => Error::MalformedMessage,
        }
    }
}

pub struct SerialProtocol<'a, PINS> {
    protocol: CommandReader,
    _serial: Serial<stm32::USART1, PINS>,

    usb_device: UsbDevice<'a, UsbBus<Peripheral>>,
    usb_serial_device: SerialPort<'a, UsbBus<Peripheral>>,
    read_buf: [u8; 64],
}

impl<'a, PINS: serial::Pins<stm32f1xx_hal::pac::USART1>> SerialProtocol<'a, PINS> {
    pub fn new(
        usart1: stm32::USART1,
        usart_pins: PINS,
        afio: &mut afio::Parts,
        apb: &mut rcc::APB2,
        clocks: rcc::Clocks,
        usb_device: usb_device::device::UsbDevice<
            'a,
            stm32f1xx_hal::usb::UsbBus<stm32f1xx_hal::usb::Peripheral>,
        >,
        usb_serial_device: usbd_serial::SerialPort<
            'a,
            stm32f1xx_hal::usb::UsbBus<stm32f1xx_hal::usb::Peripheral>,
        >,
    ) -> Self {
        let serial_config = serial::Config::default().baudrate(115200.bps());
        let mapr = &mut afio.mapr;
        let serial = serial::Serial::usart1(usart1, usart_pins, mapr, serial_config, clocks, apb);

        Self {
            protocol: CommandReader::new(),
            _serial: serial,
            usb_device,
            usb_serial_device,
            read_buf: [0u8; 64],
        }
    }

    /// Check to see if a new command from host is available
    pub fn poll(&mut self) -> Result<Option<Command>, Error> {
        self.usb_device.poll(&mut [&mut self.usb_serial_device]);

        match self.usb_serial_device.read(&mut self.read_buf) {
            Ok(count) if count > 0 => {
                for byte in &self.read_buf[..count] {
                    if let Some(command) = self.protocol.process_byte(*byte)? {
                        return Ok(Some(command));
                    }
                }

                Ok(None)
            },
            Ok(_) => Ok(None),
            Err(UsbError::WouldBlock) => Ok(None),
            Err(e) => Err(e.into()),
        }

        // loop {
        //     match self.serial.read() {
        //         Ok(byte) => {
        //             if let Some(command) = self.protocol.process_byte(byte)? {
        //                 break Ok(Some(command));
        //             }
        //         },
        //         Err(nb::Error::WouldBlock) => break Ok(None),
        //         Err(nb::Error::Other(e)) => break Err(e.into()),
        //     }
        // }
    }

    /// Sends a new report to the host, blocks until fully written or error occurs.
    pub fn report(&mut self, report: Report) -> Result<(), Error> {
        let report_bytes = report.as_arrayvec();
        let mut write_offset = 0;
        let count = report_bytes.len();

        while write_offset < count {
            match self.usb_serial_device.write(&report_bytes[write_offset..count]) {
                Ok(len) if len > 0 => {
                    write_offset += len;
                },
                _ => {},
            }
        }

        // for byte in report_bytes.into_iter() {
        //     // We can unwrap here because serial.write() returns an
        //     // Infallible error.
        //     block!(self.serial.write(byte)).unwrap();
        // }
        Ok(())
    }
}
