use stm32f1xx_hal as hal;

use hal::{
    prelude::*,
    serial::{self, Serial},
    stm32,
};
use nb::{self, block};
pub use panel_protocol::{Command, Report, Protocol};

#[derive(Debug)]
pub enum Error {
    Serial,
    BufferFull,
    MalformedMessage,
}

impl From<serial::Error> for Error {
    fn from(_: serial::Error) -> Error {
        Error::Serial
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
    protocol: Protocol,
    serial: Serial<stm32::USART1, PINS>,
}

impl<PINS> SerialProtocol<PINS> {
    pub fn new(serial: Serial<stm32::USART1, PINS>) -> Self {
        Self { protocol: Protocol::new(), serial }
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
