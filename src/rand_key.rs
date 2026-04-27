use rand::seq::{IndexedRandom, SliceRandom};
use rand::{RngExt, rng};

pub fn generate_strong_password(length: usize) -> Result<String, &'static str> {
    if length < 12 {
        return Err("length must be at least 12");
    }

    const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const DIGITS: &[u8] = b"0123456789";
    const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{};:,.?/";
    let all: Vec<u8> = [LOWER, UPPER, DIGITS, SYMBOLS].concat();

    let mut rng = rng();
    let mut pwd = vec![
        *LOWER.choose(&mut rng).unwrap(),
        *UPPER.choose(&mut rng).unwrap(),
        *DIGITS.choose(&mut rng).unwrap(),
        *SYMBOLS.choose(&mut rng).unwrap(),
    ];

    for _ in 0..(length - 4) {
        let idx = rng.random_range(0..all.len());
        pwd.push(all[idx]);
    }

    pwd.shuffle(&mut rng);
    Ok(String::from_utf8(pwd).unwrap())
}
pub fn generate_random_lower_id(length: usize) -> String {
    let charset = b"abcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::rng();
    
    (0..length)
        .map(|_| {
            // 从字母表切片中随机选择一个字符
            *charset.choose(&mut rng).unwrap() as char
        })
        .collect()
}