pub fn push_varint(out: &mut Vec<u8>, n: u64) {
    match n {
        0..=0xfc => out.push(n as u8),
        0xfd..=0xffff => {
            out.push(0xfd);
            out.extend_from_slice(&(n as u16).to_le_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(0xfe);
            out.extend_from_slice(&(n as u32).to_le_bytes());
        }
        _ => {
            out.push(0xff);
            out.extend_from_slice(&n.to_le_bytes());
        }
    }
}

pub fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    push_varint(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

#[must_use]
pub fn strip_witness(tx: &[u8]) -> Option<Vec<u8>> {
    if tx.len() < 6 || tx[4] != 0x00 || tx[5] == 0x00 {
        return None;
    }

    let mut cursor = 6;
    let (vin_count, n) = read_varint(tx, cursor)?;
    cursor = n;
    for _ in 0..vin_count {
        cursor = cursor.checked_add(36)?;
        let (script_len, n) = read_varint(tx, cursor)?;
        cursor = n.checked_add(script_len as usize)?.checked_add(4)?;
        if cursor > tx.len() {
            return None;
        }
    }

    let outputs_start = cursor;
    let (vout_count, n) = read_varint(tx, cursor)?;
    cursor = n;
    for _ in 0..vout_count {
        cursor = cursor.checked_add(8)?;
        let (script_len, n) = read_varint(tx, cursor)?;
        cursor = n.checked_add(script_len as usize)?;
        if cursor > tx.len() {
            return None;
        }
    }
    let outputs_end = cursor;

    for _ in 0..vin_count {
        let (items, n) = read_varint(tx, cursor)?;
        cursor = n;
        for _ in 0..items {
            let (item_len, n) = read_varint(tx, cursor)?;
            cursor = n.checked_add(item_len as usize)?;
            if cursor > tx.len() {
                return None;
            }
        }
    }

    let locktime = tx.get(cursor..cursor + 4)?;
    let mut stripped = Vec::new();
    stripped.extend_from_slice(&tx[0..4]);
    stripped.extend_from_slice(&tx[6..outputs_start]);
    stripped.extend_from_slice(&tx[outputs_start..outputs_end]);
    stripped.extend_from_slice(locktime);
    Some(stripped)
}

fn read_varint(data: &[u8], offset: usize) -> Option<(u64, usize)> {
    let tag = *data.get(offset)?;
    match tag {
        0x00..=0xfc => Some((tag as u64, offset + 1)),
        0xfd => Some((
            u16::from_le_bytes(data.get(offset + 1..offset + 3)?.try_into().ok()?) as u64,
            offset + 3,
        )),
        0xfe => Some((
            u32::from_le_bytes(data.get(offset + 1..offset + 5)?.try_into().ok()?) as u64,
            offset + 5,
        )),
        0xff => Some((
            u64::from_le_bytes(data.get(offset + 1..offset + 9)?.try_into().ok()?),
            offset + 9,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_varint_boundaries() {
        let mut out = Vec::new();
        push_varint(&mut out, 0xfc);
        push_varint(&mut out, 0xfd);
        assert_eq!(hex::encode(out), "fcfdfd00");
    }
}
