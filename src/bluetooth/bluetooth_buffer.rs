#[derive(Default)]
pub struct MessageBuffer {
    bytes: Vec<u8>,
}

impl MessageBuffer {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn next_message(&mut self) -> Option<Vec<u8>> {
        let new_line = self.bytes.iter().position(|byte| *byte == b'\n')?;
        let mut message: Vec<u8> = self.bytes.drain(0..=new_line).collect();

        if message.last() == Some(&b'\n') {
            message.pop();
        }

        if message.last() == Some(&b'\r') {
            message.pop();
        }

        Some(message)
    }

    pub fn append(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiple_messages() {
        let mut buffer = MessageBuffer::new();

        buffer.bytes.extend_from_slice(b"P3 0110\r\nP4 1001\r\n");

        let first = buffer.next_message().unwrap();
        let second = buffer.next_message().unwrap();

        assert_eq!(first, b"P3 0110");
        assert_eq!(second, b"P4 1001");
    }
}
