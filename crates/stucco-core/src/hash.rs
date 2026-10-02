//! Content hashing for cache-busting file names.

/// 64-bit FNV-1a.
pub(crate) fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The first 10 lowercase hex digits of the content hash.
pub(crate) fn short_hash(bytes: &[u8]) -> String {
    format!("{:016x}", fnv1a64(bytes))[..10].to_owned()
}

#[cfg(test)]
mod tests {
    #[test]
    fn matches_the_reference_vectors() {
        assert_eq!(super::fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(super::fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
