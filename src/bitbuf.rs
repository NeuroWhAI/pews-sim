pub(crate) struct BitBuf {
    buff: Vec<u8>,
    bit_len: usize,
}

impl BitBuf {
    pub(crate) fn new() -> Self {
        BitBuf {
            buff: Vec::new(),
            bit_len: 0,
        }
    }

    pub(crate) fn write_bit(&mut self, bit: bool) {
        let offset = self.bit_len % 8;
        if offset == 0 {
            self.buff.push((bit as u8) << 7);
        } else {
            let buff_len = self.buff.len();
            let latest = unsafe { self.buff.get_unchecked_mut(buff_len - 1) };
            *latest = *latest | ((bit as u8) << (7 - offset));
        }
        self.bit_len += 1;
    }

    pub(crate) fn write_int(&mut self, num: u64, lower_bits: usize) {
        let bytes = num.to_be_bytes();
        let begin = bytes.len() - 1 - (lower_bits - 1) / 8;

        let offset = lower_bits % 8;
        if offset == 0 {
            self.write_bytes(&bytes[begin..]);
        } else {
            let shifted_end = bytes.len() - begin;
            let mut shifted = [0u8; 8];
            shifted[0] = bytes[begin] << (8 - offset);

            for (i, &b) in bytes[(begin + 1)..].into_iter().enumerate() {
                shifted[i] |= b >> offset;
                shifted[i + 1] = b << (8 - offset);
            }

            self.write_bytes(&shifted[..(shifted_end - 1)]);

            for i in 0..offset {
                self.write_bit((shifted[shifted_end - 1] & (1 << (7 - i))) != 0);
            }
        }
    }

    pub(crate) fn write_string(&mut self, s: &str, fixed_size: usize) {
        let bytes = s.as_bytes();
        self.write_bytes(
            &bytes
                .into_iter()
                .copied()
                .chain(std::iter::repeat(0x20).take(fixed_size - bytes.len()))
                .collect::<Vec<_>>(),
        );
    }

    pub(crate) fn write_bytes(&mut self, bytes: &[u8]) {
        let offset = self.bit_len % 8;
        if offset == 0 {
            self.buff.extend_from_slice(bytes);
        } else {
            for &b in bytes {
                {
                    let buff_len = self.buff.len();
                    let latest = unsafe { self.buff.get_unchecked_mut(buff_len - 1) };
                    *latest = *latest | (b >> offset);
                }
                self.buff.push(b << (8 - offset));
            }
        }
        self.bit_len += bytes.len() * 8;
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.buff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_bitbuf() {
        let mut buf = BitBuf::new();

        buf.write_bit(true);
        buf.write_bit(false);
        buf.write_bit(false);
        buf.write_bit(true);
        buf.write_bytes(&[0x12, 0x34]);
        buf.write_bit(false);
        buf.write_bit(true);
        buf.write_bit(true);
        buf.write_bit(false);
        buf.write_bytes(&[0x56, 0x78]);

        buf.write_int(0x4321, 16);
        buf.write_bit(true);
        buf.write_bit(false);
        buf.write_bit(true);
        buf.write_bit(false);
        buf.write_int(0b10100110, 4);
        buf.write_int(0b10100110, 6);

        assert_eq!(
            &buf.finish(),
            &[0x91, 0x23, 0x46, 0x56, 0x78, 0x43, 0x21, 0xa6, 0x98]
        );
    }

    #[test]
    fn make_bitbuf2() {
        let mut buf = BitBuf::new();

        buf.write_int(466, 10);
        buf.write_int(240, 10);

        assert_eq!(&buf.finish(), &[0b01110100, 0b10001111, 0x00]);
    }
}
