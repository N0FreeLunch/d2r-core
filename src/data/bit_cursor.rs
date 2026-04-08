use bitstream_io::{BitRead, Numeric};
use std::io;
use crate::domain::item::{BitSegment, RecordedBit};

/// A bit-precision cursor that wraps a `BitRead` implementation and adds
/// positioning, checkpoint/rollback, and semantic recording capabilities.
pub struct BitCursor<R: BitRead> {
    inner: R,
    bit_pos: u64,
    recorded_bits: Vec<RecordedBit>,
    segments: Vec<BitSegment>,
    context_stack: Vec<(String, u64)>, // (label, start_bit)
}

impl<R: BitRead> BitCursor<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            bit_pos: 0,
            recorded_bits: Vec::new(),
            segments: Vec::new(),
            context_stack: Vec::new(),
        }
    }

    /// Returns the current bit position.
    pub fn pos(&self) -> u64 {
        self.bit_pos
    }

    /// Reads a single bit from the stream.
    pub fn read_bit(&mut self) -> io::Result<bool> {
        let bit = self.inner.read_bit()?;
        self.recorded_bits.push(RecordedBit {
            bit,
            offset: self.bit_pos,
        });
        self.bit_pos += 1;
        Ok(bit)
    }

    /// Reads multiple bits from the stream as a numeric type.
    pub fn read_bits<T: Numeric + From<u8> + std::ops::BitOrAssign + std::ops::Shl<u32, Output = T>>(&mut self, count: u32) -> io::Result<T> {
        let mut value = T::from(0u8);
        for i in 0..count {
            if self.read_bit()? {
                value |= T::from(1u8) << i;
            }
        }
        Ok(value)
    }

    /// Begins a new semantic segment.
    pub fn begin_segment(&mut self, label: &str) {
        self.context_stack.push((label.to_string(), self.bit_pos));
    }

    /// Ends the current semantic segment and records it.
    pub fn end_segment(&mut self) {
        if let Some((label, start)) = self.context_stack.pop() {
            self.segments.push(BitSegment {
                start,
                end: self.bit_pos,
                label,
                depth: self.context_stack.len(),
            });
        }
    }

    /// Returns all recorded segments.
    pub fn segments(&self) -> &[BitSegment] {
        &self.segments
    }

    /// Returns all recorded bits.
    pub fn recorded_bits(&self) -> &[RecordedBit] {
        &self.recorded_bits
    }

    /// Creates a checkpoint of the current state.
    /// Note: This currently only saves the bit position.
    /// A full rollback requires the underlying reader to be reset.
    pub fn checkpoint(&self) -> u64 {
        self.bit_pos
    }

    /// Rollback to a previous bit position.
    /// WARNING: The underlying reader must be externally synchronized or 
    /// seeked to the same position for this to be valid.
    pub fn rollback(&mut self, checkpoint: u64) {
        self.bit_pos = checkpoint;
        self.recorded_bits.retain(|rb| rb.offset < checkpoint);
        self.segments.retain(|s| s.end <= checkpoint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitstream_io::{BitReader, LittleEndian};
    use std::io::Cursor;

    #[test]
    fn test_bit_cursor_basic_read() {
        let bytes = vec![0b10110010]; // 0xB2
        let reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
        let mut cursor = BitCursor::new(reader);

        assert_eq!(cursor.read_bit().unwrap(), false); // LSB of 0xB2 is 0
        assert_eq!(cursor.pos(), 1);
        assert_eq!(cursor.read_bit().unwrap(), true);
        assert_eq!(cursor.pos(), 2);
    }

    #[test]
    fn test_bit_cursor_segments() {
        let bytes = vec![0b00000000];
        let reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
        let mut cursor = BitCursor::new(reader);

        cursor.begin_segment("Header");
        let _ = cursor.read_bits::<u32>(4).unwrap();
        cursor.end_segment();

        assert_eq!(cursor.segments().len(), 1);
        assert_eq!(cursor.segments()[0].label, "Header");
        assert_eq!(cursor.segments()[0].start, 0);
        assert_eq!(cursor.segments()[0].end, 4);
    }

    #[test]
    fn test_bit_cursor_rollback_interface() {
        let bytes = vec![0b00000000];
        let reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
        let mut cursor = BitCursor::new(reader);

        let checkpoint = cursor.checkpoint();
        let _ = cursor.read_bits::<u32>(4).unwrap();
        cursor.rollback(checkpoint);

        assert_eq!(cursor.pos(), 0);
        assert_eq!(cursor.recorded_bits().len(), 0);
    }
}
