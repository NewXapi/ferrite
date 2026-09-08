// ponytail: FNV offset basis 非 0，Default 语义错误，clippy new_without_default 显式豁免
pub struct Fnv1aHasher(u32);

const FNV_PRIME: u32 = 16777619;

impl Default for Fnv1aHasher {
    fn default() -> Self {
        Self(0x811c9dc5)
    }
}

impl Fnv1aHasher {
    pub fn new() -> Self {
        Self(0x811c9dc5)
    }

    pub fn write(&mut self, data: &[u8]) {
        let mut hash = self.0;
        for byte in data.iter() {
            hash ^= *byte as u32;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        self.0 = hash;
    }

    pub fn finish(self) -> u32 {
        self.0
    }
}
