use stm32f4xx_hal as hal;

use hal::{
    otg_fs::{UsbBus, USB},
    serial::{self},
};
use panel_protocol::{ArrayString, ArrayVec, MAX_COMMAND_LEN};
pub use panel_protocol::{Command, CommandReader, Report};
use usb_device::{device::UsbDevice, UsbError};
use usbd_serial::SerialPort;

const MAX_COMMAND_QUEUE_LEN: usize = 8;

type Stm32F4UsbDevice = stm32f4xx_hal::otg_fs::UsbBus<stm32f4xx_hal::otg_fs::USB>;

#[derive(Debug)]
pub enum Error {
    Serial(hal::serial::Error),
    UsbError(UsbError),
    BufferFull,
    MalformedMessage,
    CommandQueueFull,
    ReportQueueFull,
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
            panel_protocol::Error::CommandQueueFull => Error::CommandQueueFull,
            panel_protocol::Error::ReportQueueFull => Error::ReportQueueFull,
        }
    }
}

pub struct SerialProtocol<'a> {
    protocol: CommandReader,
    usb_device: UsbDevice<'a, UsbBus<USB>>,
    usb_serial_device: SerialPort<'a, UsbBus<USB>>,
    read_buf: [u8; MAX_COMMAND_LEN],
}

impl<'a> SerialProtocol<'a> {
    pub fn new(
        usb_device: usb_device::device::UsbDevice<'a, Stm32F4UsbDevice>,
        usb_serial_device: usbd_serial::SerialPort<'a, Stm32F4UsbDevice>,
    ) -> Self {
        Self {
            protocol: CommandReader::new(),
            usb_device,
            usb_serial_device,
            read_buf: [0u8; MAX_COMMAND_LEN],
        }
    }

    /// Check to see if a new command from host is available
    pub fn poll(&mut self) -> Result<ArrayVec<Command, MAX_COMMAND_QUEUE_LEN>, Error> {
        self.usb_device.poll(&mut [&mut self.usb_serial_device]);

        match self.usb_serial_device.read(&mut self.read_buf[..]) {
            Ok(count) if count > 0 => {
                let commands = self.protocol.process_bytes(&self.read_buf[..count])?;
                Ok(commands)
            },
            Ok(_) | Err(UsbError::WouldBlock) => Ok(ArrayVec::new()),
            Err(e) => Err(e.into()),
        }
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

        Ok(())
    }

    #[allow(dead_code)]
    pub fn debug(&mut self, message: &str) {
        let report = Report::Debug { message: ArrayString::from(message).unwrap() };
        let _ = self.report(report);
    }
}
