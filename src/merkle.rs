use crate::coinbase::WITNESS_RESERVED_VALUE;
use crate::crypto::sha256d;

#[must_use]
pub fn merkle_root(mut hashes: Vec<[u8; 32]>) -> [u8; 32] {
    if hashes.is_empty() {
        return [0u8; 32];
    }
    while hashes.len() > 1 {
        if hashes.len() % 2 == 1 {
            let last = *hashes.last().expect("non-empty");
            hashes.push(last);
        }
        hashes = hashes
            .chunks_exact(2)
            .map(|pair| {
                let mut joined = Vec::with_capacity(64);
                joined.extend_from_slice(&pair[0]);
                joined.extend_from_slice(&pair[1]);
                sha256d(&joined)
            })
            .collect();
    }
    hashes[0]
}

#[must_use]
pub fn witness_commitment(wtxids: Vec<[u8; 32]>) -> [u8; 32] {
    let witness_root = merkle_root(wtxids);
    let mut data = Vec::with_capacity(64);
    data.extend_from_slice(&witness_root);
    data.extend_from_slice(&WITNESS_RESERVED_VALUE);
    sha256d(&data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merkle_root_duplicates_odd_leaf() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let c = [3u8; 32];
        let root3 = merkle_root(vec![a, b, c]);
        let root4 = merkle_root(vec![a, b, c, c]);
        assert_eq!(root3, root4);
    }
}
