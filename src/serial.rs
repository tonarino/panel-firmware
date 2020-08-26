use stm32f1xx_hal as hal;

use hal::{
    afio,
    prelude::*,
    rcc,
    serial::{self, Serial},
    stm32,
};
use nb::{self, block};
pub use panel_protocol::{Command, CommandReader, Report};

#[derive(Debug)]
pub enum Error {
    Serial(hal::serial::Error),
    BufferFull,
    MalformedMessage,
}

impl From<serial::Error> for Error {
    fn from(e: serial::Error) -> Error {
        Error::Serial(e)
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

pub struct SerialProtocol<PINS> {
    protocol: CommandReader,
    serial: Serial<stm32::USART1, PINS>,
}

impl<PINS: serial::Pins<stm32f1xx_hal::pac::USART1>> SerialProtocol<PINS> {
    pub fn new(
        usart1: stm32::USART1,
        usart_pins: PINS,
        afio: &mut afio::Parts,
        apb: &mut rcc::APB2,
        clocks: rcc::Clocks,
    ) -> Self {
        let serial_config = serial::Config::default().baudrate(115200.bps());
        let mapr = &mut afio.mapr;
        let serial = serial::Serial::usart1(usart1, usart_pins, mapr, serial_config, clocks, apb);

        Self { protocol: CommandReader::new(), serial }
    }

    /// Check to see if a new command from host is available
    pub fn poll(&mut self) -> Result<Option<Command>, Error> {
        loop {
            match self.serial.read() {
                Ok(byte) => {
                    if let Some(command) = self.protocol.process_byte(byte)? {
                        break Ok(Some(command));
                    }
                },
                Err(nb::Error::WouldBlock) => break Ok(None),
                Err(nb::Error::Other(e)) => break Err(e.into()),
            }
        }
    }

    /// Sends a new report to the host, blocks until fully written or error occurs.
    pub fn report(&mut self, report: Report) -> Result<(), Error> {
        let report_bytes = report.as_arrayvec();
        for byte in report_bytes.into_iter() {
            // We can unwrap here because serial.write() returns an
            // Infallible error.
            block!(self.serial.write(byte)).unwrap();
        }
        Ok(())
    }
}
