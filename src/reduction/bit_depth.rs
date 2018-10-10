use bit_vec::BitVec;
use colors::{BitDepth, ColorType};
use png::PngData;

pub fn reduce_bit_depth_8_or_less(png: &mut PngData) -> bool {
    let mut reduced = BitVec::with_capacity(png.raw_data.len() * 8);
    let bit_depth: usize = png.ihdr_data.bit_depth.as_u8() as usize;
    let mut allowed_bits = 1;
    for line in png.scan_lines() {
        let bit_vec = BitVec::from_bytes(&line.data);
        if png.ihdr_data.color_type == ColorType::Indexed {
            for (i, bit) in bit_vec.iter().enumerate() {
                let bit_index = bit_depth - (i % bit_depth);
                if bit && bit_index > allowed_bits {
                    allowed_bits = bit_index.next_power_of_two();
                    if allowed_bits == bit_depth {
                        // Not reducable
                        return false;
                    }
                }
            }
        } else {
            for byte in bit_vec.to_bytes() {
                while allowed_bits < bit_depth {
                    let permutations = if allowed_bits == 1 {
                        vec![0b00000000, 0b11111111]
                    } else if allowed_bits == 2 {
                        vec![0b00000000, 0b00001111, 0b11110000, 0b11111111]
                    } else if allowed_bits == 4 {
                        vec![0b00000000, 0b00000011, 0b00001100, 0b00110000, 0b11000000, 0b00001111, 0b00111100, 0b11110000, 0b11111111]
                    } else {
                        unreachable!()
                    };
                    if permutations.iter().any(|perm| *perm == byte) {
                        break;
                    } else {
                        allowed_bits <<= 1;
                    }
                }
            }
        }
    }

    for line in png.scan_lines() {
        reduced.extend(BitVec::from_bytes(&[line.filter]));
        let bit_vec = BitVec::from_bytes(&line.data);
        for (i, bit) in bit_vec.iter().enumerate() {
            let bit_index = bit_depth - (i % bit_depth);
            if bit_index <= allowed_bits {
                reduced.push(bit);
            }
        }
        // Pad end of line to get 8 bits per byte
        while reduced.len() % 8 != 0 {
            reduced.push(false);
        }
    }

    png.raw_data = reduced.to_bytes();
    png.ihdr_data.bit_depth = BitDepth::from_u8(allowed_bits as u8);
    true
}
