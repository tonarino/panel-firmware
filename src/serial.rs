use stm32f1xx_hal as hal;

use core::fmt::Write;
use cortex_m::singleton;
use hal::{
    afio,
    dma::{dma1::C5, CircBuffer, Half},
    prelude::*,
    rcc,
    serial::{self, RxDma1, Tx},
    stm32,
};
use nb::{self, block};
pub use panel_protocol::{Command, CommandReader, Report};

const DMA_BUF_SIZE: usize = 1;

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

pub struct SerialProtocol {
    protocol: CommandReader,
    tx: Tx<stm32::USART1>,
    rx_buf: CircBuffer<[u8; DMA_BUF_SIZE], RxDma1>,
    next_half_to_read: Half,
}

impl SerialProtocol {
    pub fn new<PINS: serial::Pins<stm32f1xx_hal::pac::USART1>>(
        usart1: stm32::USART1,
        dma_channel_5: C5,
        usart_pins: PINS,
        afio: &mut afio::Parts,
        apb: &mut rcc::APB2,
        clocks: rcc::Clocks,
    ) -> Self {
        let serial_config = serial::Config::default().baudrate(9600.bps());
        let mapr = &mut afio.mapr;
        let serial = serial::Serial::usart1(usart1, usart_pins, mapr, serial_config, clocks, apb);
        let (tx, rx) = serial.split();

        let buf = singleton!(: [[u8; DMA_BUF_SIZE]; 2] = [[0; DMA_BUF_SIZE]; 2]).unwrap();
        let rx = rx.with_dma(dma_channel_5);
        let rx_buf = rx.circ_read(buf);

        let next_half_to_read = Half::First;

        Self { protocol: CommandReader::new(), tx, rx_buf, next_half_to_read }
    }

    fn read_byte(&mut self) -> Option<u8> {
        // TODO - instead of checking the next half to read, maybe we just call "peek",
        //        and keep track of how many bytes have been read and which read half we're
        //        currently on.
        let next_half_to_read = self.next_half_to_read;

        let peek_result = self.rx_buf.peek(|buf, read_half| match (read_half, next_half_to_read) {
            (Half::First, Half::First) => (Some(buf[0]), Half::Second),
            (Half::Second, Half::Second) => (Some(buf[0]), Half::First),
            _ => (None, next_half_to_read),
        });

        match peek_result {
            Ok((Some(byte), next_half_to_read)) => {
                self.next_half_to_read = next_half_to_read;
                Some(byte)
            },
            Ok(_) => None,
            Err(_e) => None,
        }
    }

    /// Check to see if a new command from host is available
    pub fn poll(&mut self) -> Result<Option<Command>, Error> {
        if let Some(byte) = self.read_byte() {
            if let Some(command) = self.protocol.process_byte(byte)? {
                Ok(Some(command))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }

        // loop {
        //     match self.rx.read() {
        //         Ok(byte) => {
        //             if let Some(command) = self.protocol.process_byte(byte)? {
        //                 break Ok(Some(command));
        //             }
        //         },
        //         Err(nb::Error::WouldBlock) => break Ok(None),
        //         Err(nb::Error::Other(e)) => break Err(e.into()),
        //     }
        // }
        // Ok(None)
    }

    /// Sends a new report to the host, blocks until fully written or error occurs.
    pub fn report(&mut self, report: Report) -> Result<(), Error> {
        let report_bytes = report.as_arrayvec();
        for byte in report_bytes.into_iter() {
            // We can unwrap here because serial.write() returns an
            // Infallible error.
            block!(self.tx.write(byte)).unwrap();
        }
        Ok(())
    }
}
