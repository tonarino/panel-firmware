#![cfg_attr(not(feature = "std"), no_std)]

pub use arrayvec::{ArrayString, ArrayVec};

#[derive(Debug, PartialEq)]
pub enum Command {
    PowerCycler { slot: u8, state: bool },
    Brightness { target: u8, value: u16 },
    Temperature { target: u8, value: u16 },
}

#[derive(Debug)]
pub enum Error {
    BufferFull,
    MalformedMessage,
}

#[cfg(feature = "std")]
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

// Rust doesn't support max() as a const fn, but this should be
// cmp::max(MAX_COMMAND_LEN, MAX_REPORT_LEN)
pub const MAX_SERIAL_MESSAGE_LEN: usize = 256;

pub const MAX_COMMAND_LEN: usize = 8;
pub const MAX_REPORT_LEN: usize = 256;
pub const MAX_DEBUG_MSG_LEN: usize = MAX_REPORT_LEN - 2;
pub const MAX_REPORT_QUEUE_LEN: usize = 6;
pub const MAX_COMMAND_QUEUE_LEN: usize = 6;

impl Command {
    pub fn try_from(buf: &[u8]) -> Result<Option<(Command, usize)>, ()> {
        if buf.is_empty() {
            return Ok(None);
        }

        match *buf {
            [] => Ok(None),
            [b'A', slot, state, ..] => {
                Ok(Some((Command::PowerCycler { slot, state: state != 0 }, 3)))
            },
            [b'B', target, msb, lsb, ..] => {
                let value = u16::from_be_bytes([msb, lsb]);
                Ok(Some((Command::Brightness { target, value }, 4)))
            },
            [b'C', target, msb, lsb, ..] => {
                let value = u16::from_be_bytes([msb, lsb]);
                Ok(Some((Command::Temperature { target, value }, 4)))
            },
            [header, ..] if b"ABC".contains(&header) => Ok(None),
            _ => Err(()),
        }
    }

    pub fn as_arrayvec(&self) -> ArrayVec<[u8; MAX_COMMAND_LEN]> {
        let mut buf = ArrayVec::new();
        match *self {
            Command::PowerCycler { slot, state } => {
                buf.push(b'A');
                buf.push(slot);
                buf.push(u8::from(state));
            },
            Command::Brightness { target, value } => {
                buf.push(b'B');
                buf.push(target);
                buf.try_extend_from_slice(&value.to_be_bytes()).unwrap();
            },
            Command::Temperature { target, value } => {
                buf.push(b'C');
                buf.push(target);
                buf.try_extend_from_slice(&value.to_be_bytes()).unwrap();
            },
        }
        buf
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, PartialEq)]
pub enum Report {
    Heartbeat,
    DialValue { diff: i8 },
    Press,
    LongPress,
    EmergencyOff,
    Error { code: u16 },
    Debug { message: ArrayString<[u8; MAX_DEBUG_MSG_LEN]> },
}

impl Report {
    pub fn try_from(buf: &[u8]) -> Result<Option<(Report, usize)>, ()> {
        if buf.is_empty() {
            return Ok(None);
        }

        match *buf {
            [] => Ok(None),
            [b'H', ..] => Ok(Some((Report::Heartbeat, 1))),
            [b'V', diff, ..] => {
                let diff = i8::from_be_bytes([diff]);
                Ok(Some((Report::DialValue { diff }, 2)))
            },
            [b'P', ..] => Ok(Some((Report::Press, 1))),
            [b'L', ..] => Ok(Some((Report::LongPress, 1))),
            [b'X', ..] => Ok(Some((Report::EmergencyOff, 1))),
            [b'E', msb, lsb, ..] => {
                let code = u16::from_be_bytes([msb, lsb]);
                Ok(Some((Report::Error { code }, 3)))
            },
            [b'D', len, ref message @ ..] if message.len() as u8 == len => Ok(Some((
                Report::Debug {
                    message: ArrayString::from(&core::str::from_utf8(message).unwrap()).unwrap(),
                },
                2 + message.len(),
            ))),
            [header, ..] if b"VPLXED".contains(&header) => Ok(None),
            _ => Err(()),
        }
    }

    pub fn as_arrayvec(&self) -> ArrayVec<[u8; MAX_REPORT_LEN]> {
        let mut buf = ArrayVec::new();
        match *self {
            Report::Heartbeat => {
                buf.push(b'H');
            },
            Report::DialValue { diff } => {
                buf.push(b'V');
                buf.try_extend_from_slice(&diff.to_be_bytes()).unwrap();
            },
            Report::Press => {
                buf.push(b'P');
            },
            Report::LongPress => {
                buf.push(b'L');
            },
            Report::EmergencyOff => {
                buf.push(b'X');
            },
            Report::Error { code } => {
                buf.push(b'E');
                buf.try_extend_from_slice(&code.to_be_bytes()).unwrap();
            },
            Report::Debug { ref message } => {
                buf.push(b'D');
                buf.push(message.len() as u8);
                buf.try_extend_from_slice(message.as_bytes()).unwrap();
            },
        }
        buf
    }
}

pub struct ReportReader {
    pub buf: ArrayVec<[u8; MAX_SERIAL_MESSAGE_LEN]>,
}

impl ReportReader {
    pub fn new() -> Self {
        Self { buf: ArrayVec::new() }
    }

    pub fn process_bytes(&mut self, bytes: &[u8]) -> Result<ArrayVec<[Report; MAX_REPORT_QUEUE_LEN]>, Error> {
        self.buf.try_extend_from_slice(bytes).map_err(|_| Error::BufferFull)?;

        let mut output = ArrayVec::new();

        loop {
            match Report::try_from(&self.buf[..]) {
                Ok(Some((report, bytes_read))) => {
                    self.buf.drain(0..bytes_read);
                    output.push(report);
                }
                Err(_) => return Err(Error::MalformedMessage),
                Ok(None) => break,
            }
        }

        Ok(output)
    }
}

impl Default for ReportReader {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CommandReader {
    buf: ArrayVec<[u8; MAX_SERIAL_MESSAGE_LEN]>,
}

impl CommandReader {
    pub fn new() -> Self {
        Self { buf: ArrayVec::new() }
    }

    pub fn process_bytes(&mut self, bytes: &[u8]) -> Result<ArrayVec<[Command; MAX_COMMAND_QUEUE_LEN]>, Error> {
        self.buf.try_extend_from_slice(bytes).map_err(|_| Error::BufferFull)?;

        let mut output = ArrayVec::new();

        loop {
            match Command::try_from(&self.buf[..]) {
                Ok(Some((command, bytes_read))) => {
                    self.buf.drain(0..bytes_read);
                    output.push(command);
                }
                Err(_) => return Err(Error::MalformedMessage),
                Ok(None) => break,
            }
        }

        Ok(output)
    }
}

impl Default for CommandReader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_roundtrips_arrayvec() {
        let commands = [
            Command::PowerCycler { slot: 1, state: true },
            Command::PowerCycler { slot: 20, state: false },
            Command::Temperature { target: 2, value: 100 },
            Command::Brightness { target: 10, value: 100 },
        ];

        for command in commands.iter() {
            let (deserialized, _len) =
                Command::try_from(&command.as_arrayvec()[..]).unwrap().unwrap();
            assert_eq!(command, &deserialized);
        }
    }

    #[test]
    fn report_roundtrips_arrayvec() {
        let reports = [
            Report::Press,
            Report::LongPress,
            Report::DialValue { diff: 100 },
            Report::EmergencyOff,
            Report::Error { code: 80 },
            Report::Debug { message: ArrayString::from("the frequency is 1000000000Hz").unwrap() },
        ];

        for report in reports.iter() {
            let (deserialized, _len) =
                Report::try_from(&report.as_arrayvec()[..]).unwrap().unwrap();
            assert_eq!(report, &deserialized);
        }
    }

    #[test]
    fn report_protocol_parse() {
        let reports = [
            Report::Heartbeat,
            Report::Press,
            Report::LongPress,
            Report::DialValue { diff: 100 },
            Report::EmergencyOff,
            Report::Error { code: 80 },
        ];

        let mut bytes: ArrayVec<[u8; MAX_SERIAL_MESSAGE_LEN]> = ArrayVec::new();
        for report in reports.iter() {
            bytes.try_extend_from_slice(&report.as_arrayvec()[..]).unwrap();
        }

        let mut protocol = ReportReader::new();
        let report_output = protocol.process_bytes(&bytes).unwrap();

        assert_eq!(&report_output[..], &reports[..]);
    }

    #[test]
    fn command_protocol_parse() {
        let commands = [
            Command::PowerCycler { slot: 1, state: true },
            Command::PowerCycler { slot: 20, state: false },
            Command::Temperature { target: 2, value: 100 },
            Command::Brightness { target: 10, value: 100 },
        ];

        let mut bytes: ArrayVec<[u8; MAX_SERIAL_MESSAGE_LEN]> = ArrayVec::new();
        for command in commands.iter() {
            bytes.try_extend_from_slice(&command.as_arrayvec()[..]).unwrap();
        }

        let mut protocol = CommandReader::new();
        let command_output = protocol.process_bytes(&bytes).unwrap();

        assert_eq!(&command_output[..], &commands[..]);
    }
}
