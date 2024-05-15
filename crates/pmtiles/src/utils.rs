pub const TILES_PER_LEVEL: [u64; 27] = [
    0,
    1,
    5,
    21,
    85,
    341,
    1365,
    5461,
    21845,
    87381,
    349525,
    1398101,
    5592405,
    22369621,
    89478485,
    357913941,
    1431655765,
    5726623061,
    22906492245,
    91625968981,
    366503875925,
    1466015503701,
    5864062014805,
    23456248059221,
    93824992236885,
    375299968947541,
    1501199875790165,
];

pub fn read_varint(data: &[u8], pos: &mut usize) -> anyhow::Result<u64> {
    if *pos >= data.len() {
        return Err(anyhow::anyhow!("out-of-bounds data access"));
    }
    let mut b = data[*pos] as u64;
    *pos += 1;
    let mut val = b & 0x7f;
    if b < 0x80 {
        return Ok(val as u64);
    }
    for i in 1..4 {
        if *pos >= data.len() {
            return Err(anyhow::anyhow!("out-of-bounds data access"));
        }
        b = data[*pos] as u64;
        *pos += 1;
        val |= (b & 0x7f) << (7 * (i));
        if b < 0x80 {
            return Ok(val as u64);
        }
    }
    b = data[*pos] as u64;
    val |= (b & 0x0f) << 28;
    read_varint_remainder(data, pos, val)
}

fn to_num(low: u64, high: u64) -> u64 {
    high << 32 | low
}

fn read_varint_remainder(data: &[u8], pos: &mut usize, val: u64) -> anyhow::Result<u64> {
    let mut b = data[*pos] as u64;
    *pos += 1;
    let mut high: u64 = (b & 0x70) >> 4;
    if b < 0x80 {
        return Ok(to_num(val, high));
    }
    b = data[*pos] as u64;
    *pos += 1;
    high |= (b & 0x7f) << 3;

    for i in 1..5 {
        if b < 0x80 {
            return Ok(to_num(val, high));
        }
        if *pos >= data.len() {
            return Err(anyhow::anyhow!("out-of-bounds data access"));
        }
        b = data[*pos] as u64;
        *pos += 1;
        high |= ((b & 0x7f) as u64) << (3 + 7 * i);
    }
    if b < 0x80 {
        return Ok(to_num(val, high));
    }
    Err(anyhow::anyhow!("expected varint not more than 10 bytes"))
}

pub fn rotate(n: i64, x: &mut i64, y: &mut i64, rx: i64, ry: i64) {
    if ry == 0 {
        if rx == 1 {
            *x = n - 1 - *x;
            *y = n - 1 - *y;
        }
        let tmp_x = x.clone();
        *x = *y;
        *y = tmp_x;
    }
}

#[test]
fn test_read_varint_remainder() {
    let data: Vec<u8> = vec![
        28, 0, 4, 14, 57, 229, 1, 146, 7, 199, 28, 156, 114, 242, 200, 3, 200, 163, 14, 159, 142,
        57, 253, 184, 228, 1, 244, 227, 145, 7, 1, 205, 143, 199, 28, 3, 1, 1, 179, 190, 156, 114,
        1, 1, 1, 10, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 201, 3, 229, 5, 134, 7, 153, 13, 220, 25, 168, 46, 177, 87, 154, 151,
        1, 141, 222, 1, 232, 153, 2, 234, 207, 2, 251, 131, 3, 184, 193, 2, 170, 188, 1, 188, 218,
        1, 215, 163, 1, 184, 93, 166, 122, 99, 171, 131, 2, 227, 8, 99, 196, 2, 202, 183, 1, 215,
        4, 137, 5, 197, 104, 189, 141, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let mut pos = 27;
    let val = 1077484669;
    let result = read_varint_remainder(&data, &mut pos, val).unwrap();
    assert_eq!(result, 31397288418429);
}

#[test]
fn test_varint() {
    let mut pos = 0;
    let data: Vec<u8> = vec![0, 1, 127, 0xe5, 0x8e, 0x26];
    let v = read_varint(&data, &mut pos).unwrap();
    assert_eq!(v, 0);

    let v = read_varint(&data, &mut pos).unwrap();
    assert_eq!(v, 1);

    let v = read_varint(&data, &mut pos).unwrap();
    assert_eq!(v, 127);

    let v = read_varint(&data, &mut pos).unwrap();
    assert_eq!(v, 624485);

    let data: Vec<u8> = vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x0f];
    pos = 0;
    let v = read_varint(&data, &mut pos).unwrap();
    assert_eq!(v, 9007199254740991);

    let data: Vec<u8> = vec![
        28, 0, 4, 14, 57, 229, 1, 146, 7, 199, 28, 156, 114, 242, 200, 3, 200, 163, 14, 159, 142,
        57, 253, 184, 228, 1, 244, 227, 145, 7, 1, 205, 143, 199, 28, 3, 1, 1, 179, 190, 156, 114,
        1, 1, 1, 10, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 1, 1, 1, 201, 3, 229, 5, 134, 7, 153, 13, 220, 25, 168, 46, 177, 87, 154, 151,
        1, 141, 222, 1, 232, 153, 2, 234, 207, 2, 251, 131, 3, 184, 193, 2, 170, 188, 1, 188, 218,
        1, 215, 163, 1, 184, 93, 166, 122, 99, 171, 131, 2, 227, 8, 99, 196, 2, 202, 183, 1, 215,
        4, 137, 5, 197, 104, 189, 141, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    pos = 22;
    let v = read_varint(&data, &mut pos).unwrap();
    assert_eq!(v, 3742845);
}
