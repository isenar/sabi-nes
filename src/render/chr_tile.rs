use crate::Byte;

pub struct ChrTile(pub [Byte; 16]);

impl ChrTile {
    pub fn pixel(&self, x: usize, y: usize) -> Byte {
        debug_assert!(x <= 7);

        let bit = 7 - x; // NES CHR: bit 7 is the leftmost pixel
        let low = self.0[y];
        let high = self.0[y + 8];
        (((high >> bit) & 1) << 1) | ((low >> bit) & 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Byte;

    fn tile_with(low_plane: [u8; 8], high_plane: [u8; 8]) -> ChrTile {
        let mut data = [Byte::new(0); 16];
        for i in 0..8 {
            data[i] = Byte::new(low_plane[i]);
            data[i + 8] = Byte::new(high_plane[i]);
        }
        ChrTile(data)
    }

    #[test]
    fn transparent_pixel_when_both_planes_zero() {
        let tile = tile_with([0; 8], [0; 8]);
        assert_eq!(tile.pixel(0, 0), Byte::new(0));
    }

    #[test]
    fn low_plane_bit_gives_value_one() {
        // bit 7 of low plane = leftmost pixel (x=0)
        let tile = tile_with([0b1000_0000, 0, 0, 0, 0, 0, 0, 0], [0; 8]);
        assert_eq!(tile.pixel(0, 0), Byte::new(1));
    }

    #[test]
    fn high_plane_bit_gives_value_two() {
        // bit 7 of high plane = leftmost pixel (x=0), high bit set = value 2
        let tile = tile_with([0; 8], [0b1000_0000, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(tile.pixel(0, 0), Byte::new(2));
    }

    #[test]
    fn both_planes_set_gives_value_three() {
        let tile = tile_with(
            [0b1000_0000, 0, 0, 0, 0, 0, 0, 0],
            [0b1000_0000, 0, 0, 0, 0, 0, 0, 0],
        );
        assert_eq!(tile.pixel(0, 0), Byte::new(3));
    }

    #[test]
    fn rightmost_pixel_reads_bit_zero() {
        // x=7 → bit 0 of each plane
        let tile = tile_with([0b0000_0001, 0, 0, 0, 0, 0, 0, 0], [0; 8]);
        assert_eq!(tile.pixel(7, 0), Byte::new(1));
    }

    #[test]
    fn pixel_reads_correct_row() {
        // Only row 3 has data
        let mut low = [0u8; 8];
        low[3] = 0b1000_0000;
        let tile = tile_with(low, [0; 8]);
        assert_eq!(tile.pixel(0, 3), Byte::new(1));
        assert_eq!(tile.pixel(0, 0), Byte::new(0));
        assert_eq!(tile.pixel(0, 7), Byte::new(0));
    }
}
