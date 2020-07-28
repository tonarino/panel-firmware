use stm32f1xx_hal as hal;

use arrayvec::ArrayVec;
use hal::{
    prelude::*,
    serial::{self, Serial},
    stm32,
};
use nb::{self, block};
pub use panel_protocol::{Command, Report};

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

impl From<arrayvec::CapacityError> for Error {
    fn from(_: arrayvec::CapacityError) -> Error {
        Error::BufferFull
    }
}

pub struct Protocol<PINS> {
    buf: ArrayVec<[u8; 256]>,
    serial: Serial<stm32::USART1, PINS>,
}

impl<PINS> Protocol<PINS> {
    pub fn new(serial: Serial<stm32::USART1, PINS>) -> Self {
        Self { buf: ArrayVec::new(), serial }
    }

    fn process_byte(&mut self, byte: u8) -> Result<Option<Command>, Error> {
        self.buf.try_push(byte).map_err(|_| serial::Error::Overrun)?;
        match Command::try_from(&self.buf[..]) {
            Ok(Some((command, bytes_read))) => {
                self.buf.drain(0..bytes_read);
                Ok(Some(command))
            },
            Err(_) => Err(Error::MalformedMessage),
            Ok(None) => Ok(None),
        }
    }

    /// Check to see if a new command from host is available
    pub fn poll(&mut self) -> Result<Option<Command>, Error> {
        loop {
            match self.serial.read() {
                Ok(byte) => {
                    if let Some(command) = self.process_byte(byte)? {
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
