pub fn from_hex(input: &str) -> Result<u8, std::num::ParseIntError> {
    u8::from_str_radix(input, 16)
}

pub fn is_hex_digit(c: char) -> bool {
    c.is_digit(16)
}
