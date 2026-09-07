//! Minimal SHA-1 (FIPS 180-1) used to verify the downloaded tag DB against
//! the `.sha1` file the upstream publishes, mirroring the original app's
//! `MessageDigest.getInstance("SHA-1")` check. Deliberately dependency-free.

const H0: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];

pub fn sha1_hex(data: &[u8]) -> String {
    let mut h = H0;
    let bit_len = (data.len() as u64).wrapping_mul(8);

    let mut block = [0u8; 64];
    let mut chunks = data.chunks_exact(64);
    for chunk in &mut chunks {
        block.copy_from_slice(chunk);
        compress(&mut h, &block);
    }

    // Padding: 0x80, zeros, then the 64-bit big-endian bit length.
    let rem = chunks.remainder();
    let mut tail = [0u8; 128];
    tail[..rem.len()].copy_from_slice(rem);
    tail[rem.len()] = 0x80;
    let tail_len = if rem.len() < 56 { 64 } else { 128 };
    tail[tail_len - 8..tail_len].copy_from_slice(&bit_len.to_be_bytes());
    for chunk in tail[..tail_len].chunks_exact(64) {
        block.copy_from_slice(chunk);
        compress(&mut h, &block);
    }

    let mut out = String::with_capacity(40);
    for word in h {
        use std::fmt::Write as _;
        let _ = write!(out, "{word:08x}");
    }
    out
}

fn compress(h: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0u32; 80];
    for (i, chunk) in block.chunks_exact(4).enumerate() {
        w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }

    let [mut a, mut b, mut c, mut d, mut e] = *h;
    for (i, &wi) in w.iter().enumerate() {
        let (f, k) = match i {
            0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
            20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
            _ => (b ^ c ^ d, 0xCA62C1D6),
        };
        let tmp = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(wi);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = tmp;
    }

    h[0] = h[0].wrapping_add(a);
    h[1] = h[1].wrapping_add(b);
    h[2] = h[2].wrapping_add(c);
    h[3] = h[3].wrapping_add(d);
    h[4] = h[4].wrapping_add(e);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        assert_eq!(sha1_hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            sha1_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
        // One million 'a' characters.
        let million_a = vec![b'a'; 1_000_000];
        assert_eq!(
            sha1_hex(&million_a),
            "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
        );
    }

    #[test]
    fn cross_block_boundaries() {
        // 55/56/64/119-byte inputs exercise both padding tail layouts.
        for n in [55usize, 56, 57, 63, 64, 65, 119, 120, 128] {
            let data = vec![0xabu8; n];
            let _ = sha1_hex(&data); // must not panic
        }
        assert_eq!(
            sha1_hex(&[b'a'; 55]),
            "c1c8bbdc22796e28c0e15163d20899b65621d65a"
        );
        assert_eq!(
            sha1_hex(&[b'a'; 56]),
            "c2db330f6083854c99d4b5bfb6e8f29f201be699"
        );
    }
}
