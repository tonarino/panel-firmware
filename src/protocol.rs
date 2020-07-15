use arrayvec::ArrayVec;
use stm32f4xx_hal::{
    nb::{self, block},
    prelude::*,
    serial::{self, Serial},
    stm32,
};

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

#[derive(Debug, PartialEq)]
pub enum Command {
    PowerCycler { slot: u8, state: bool },
    Brightness { value: u16 },
    Temperature { value: u16 },
}

impl Command {
    pub fn try_from(buf: &[u8]) -> Result<Option<(Command, usize)>, ()> {
        if buf.is_empty() {
            return Ok(None);
        }

        match *buf {
            [] => Ok(None),
            [0, slot, state, ..] => Ok(Some((Command::PowerCycler { slot, state: state != 0 }, 3))),
            [1, msb, lsb, ..] => {
                let value = u16::from_be_bytes([msb, lsb]);
                Ok(Some((Command::Brightness { value }, 3)))
            },
            [2, msb, lsb, ..] => {
                let value = u16::from_be_bytes([msb, lsb]);
                Ok(Some((Command::Temperature { value }, 3)))
            },
            _ => Err(()),
        }
    }

    pub fn as_arrayvec(&self) -> ArrayVec<[u8; 8]> {
        let mut buf = ArrayVec::new();
        match *self {
            Command::PowerCycler { slot, state } => {
                buf.push(0);
                buf.push(slot);
                buf.push(u8::from(state));
            },
            Command::Brightness { value } => {
                buf.push(1);
                buf.try_extend_from_slice(&value.to_be_bytes()).unwrap();
            }
            Command::Temperature { value } => {
                buf.push(2);
                buf.try_extend_from_slice(&value.to_be_bytes()).unwrap();
            }
        }
        buf
    }
}

#[derive(Debug, PartialEq)]
pub enum Report {
    DialValue { diff: i8 },
    Click,
    EmergencyOff,
    Error { code: u16 },
}

impl Report {
    pub fn try_from(buf: &[u8]) -> Result<Option<(Report, usize)>, ()> {
        if buf.is_empty() {
            return Ok(None);
        }

        match *buf {
            [] => Ok(None),
            [0, diff, ..] => {
                let diff = i8::from_be_bytes([diff]);
                Ok(Some((Report::DialValue { diff }, 2)))
            },
            [1, ..] => {
                Ok(Some((Report::Click, 1)))
            },
            [2, ..] => {
                Ok(Some((Report::EmergencyOff, 1)))
            },
            [3, msb, lsb, ..] => {
                let code = u16::from_be_bytes([msb, lsb]);
                Ok(Some((Report::Error { code }, 3)))
            },
            _ => Err(()),
        }
    }

    pub fn as_arrayvec(&self) -> ArrayVec<[u8; 8]> {
        let mut buf = ArrayVec::new();
        match *self {
            Report::DialValue { diff } => {
                buf.push(0);
                buf.try_extend_from_slice(&diff.to_be_bytes()).unwrap();
            },
            Report::Click => {
                buf.push(1);
            }
            Report::EmergencyOff => {
                buf.push(2);
            }
            Report::Error { code } => {
                buf.push(3);
                buf.try_extend_from_slice(&code.to_be_bytes()).unwrap();
            }
        }
        buf
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
            Err(_) => {
                Err(Error::MalformedMessage)
            },
            Ok(None) => Ok(None),
        }
    }

    /// Check to see if a new command from host is available
    pub fn poll(&mut self) -> Result<Option<Command>, Error> {
        loop {
            match self.serial.read() {
                Ok(byte) => {
                    if let Some(command) = self.process_byte(byte)? {
                        break Ok(Some(command))
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
            block!(self.serial.write(byte))?;
        }
        Ok(())
    }
}
