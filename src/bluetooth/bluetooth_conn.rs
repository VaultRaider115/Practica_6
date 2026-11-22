use super::bluetooth_buffer::MessageBuffer;
use bluer::rfcomm::Stream;
use std::io::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};

pub struct BluetoothConnection {
    rx: ReadHalf<Stream>,
    tx: WriteHalf<Stream>,
    buffer: MessageBuffer,
}

pub struct BluetoothReader {
    rx: ReadHalf<Stream>,
    buffer: MessageBuffer,
}

pub struct BluetoothWriter {
    tx: WriteHalf<Stream>,
}

impl BluetoothReader {
    async fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Error> {
        self.rx.read(buffer).await
    }

    pub async fn receive_message(&mut self) -> Result<Vec<u8>, Error> {
        let mut buf = [0u8; 256];

        loop {
            if let Some(message) = self.buffer.next_message() {
                return Ok(message);
            }

            let cantidad = self.receive(&mut buf).await?;

            if cantidad == 0 {
                return Err(Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Bluetooth connection closed",
                ));
            }

            self.buffer.append(&buf[..cantidad]);
        }
    }
}

impl BluetoothWriter {
    pub async fn send(&mut self, message: u8) -> Result<(), Error> {
        self.tx.write_all(&[message]).await?;
        Ok(())
    }
}

impl BluetoothConnection {
    pub fn new(stream: Stream) -> Self {
        let (rxs, txs) = tokio::io::split(stream);
        Self {
            rx: rxs,
            tx: txs,
            buffer: MessageBuffer::new(),
        }
    }

    pub fn into_parts(self) -> (BluetoothReader, BluetoothWriter) {
        let BluetoothConnection { rx, tx, buffer } = self;
        (BluetoothReader { rx, buffer }, BluetoothWriter { tx })
    }
}
