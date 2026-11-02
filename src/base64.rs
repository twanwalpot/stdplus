//! Simple base64 encoding and decoding implementation.
//!
//!
//! <https://datatracker.ietf.org/doc/html/rfc4648>

const PADDING_DEFAULT: u8 = b'=';

const ALPHABET_STANDARD: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

const STANDARD: Config = match Config::new(ALPHABET_STANDARD, PADDING_DEFAULT) {
    Ok(config) => config,
    Err(_) => panic!("invalid standard config"),
};

const ALPHABET_URLSAFE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

const URLSAFE: Config = match Config::new(ALPHABET_URLSAFE, PADDING_DEFAULT) {
    Ok(config) => config,
    Err(_) => panic!("invalid standard config"),
};

/// Used to drive encoding and decoding.
/// It is constructed from an alphabet and padding character.
#[derive(Debug, PartialEq)]
pub struct Config {
    encode_table: &'static [u8; 64],
    decode_table: [u8; 256],
    padding: u8,
}

/// The error returned on invalid config construction.
#[derive(Debug, PartialEq)]
pub enum ConfigError {
    /// The alphabet contains a duplicate character e.g. two times 'R'.
    Duplicate(u8),
    /// The alphabet contains a reserved value. Currently only the padding.
    Reserved(u8),
}

/// The error returned on invalid decoder input.
#[derive(Debug, PartialEq)]
pub enum DecodeError {
    /// A character in the input does not match a symbol in the decode table.
    Invalid(u8),
    /// The input either contains more than 2 padding characters or it contains
    /// padding characters when it should not i.e. input length is divisible by 4.
    InvalidPadding,
    /// The input length is not exactly a multiple of 4.
    InvalidLength,
    /// The input contains non-zero bits after the last encoded byte.
    TrailingBits,
}

/// Encode `input` using the standard base64 configuration.
pub fn encode(input: impl AsRef<[u8]>) -> String {
    encode_with_config(input.as_ref(), &STANDARD)
}

/// Decode `input` using the standard base64 configuration.
pub fn decode(input: impl AsRef<str>) -> Result<Vec<u8>, DecodeError> {
    decode_with_config(input.as_ref(), &STANDARD)
}

/// Encode `input` using the urlsafe base64 configuration.
/// Urlsafe means `/` and `+` from standard are replaced with `_` and `+`.
pub fn encode_urlsafe(input: impl AsRef<[u8]>) -> String {
    encode_with_config(input.as_ref(), &URLSAFE)
}

/// Decode `input` using the urlsafe base64 configuration.
/// Urlsafe means `/` and `+` from standard are replaced with `_` and `+`.
pub fn decode_urlsafe(input: impl AsRef<str>) -> Result<Vec<u8>, DecodeError> {
    decode_with_config(input.as_ref(), &URLSAFE)
}

/// Encode `input` using an explicit configuration.
pub fn encode_with_config(input: &[u8], config: &Config) -> String {
    let capacity = input.len().div_ceil(3) * 4;

    let mut output = String::with_capacity(capacity);

    let mut groups = input.chunks_exact(3);
    for group in &mut groups {
        encode_group(config.encode_table, group, &mut output);
    }

    let remainder = groups.remainder();
    if !remainder.is_empty() {
        encode_group(config.encode_table, remainder, &mut output);
        output.extend(core::iter::repeat_n(
            config.padding as char,
            3 - groups.remainder().len(),
        ));
    }

    output
}

fn encode_group(encode_table: &[u8; 64], group: &[u8], output: &mut String) {
    debug_assert!(!group.is_empty() && group.len() <= 3);

    let n = group
        .iter()
        .enumerate()
        .fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));

    output.extend(
        [18, 12, 6, 0]
            .into_iter()
            .take(group.len() + 1)
            .map(|shift| encode_table[(n >> shift & 0x3F) as usize] as char),
    );
}

/// Decode `input` using an explicit configuration.
pub fn decode_with_config(input: &str, config: &Config) -> Result<Vec<u8>, DecodeError> {
    if !input.len().is_multiple_of(4) {
        return Err(DecodeError::InvalidLength);
    }

    let input_without_padding = input.trim_end_matches(config.padding as char);

    if input.len() - input_without_padding.len() > 2
        || input_without_padding.contains(config.padding as char)
    {
        return Err(DecodeError::InvalidPadding);
    }

    let data = input_without_padding.as_bytes();

    let mut output = Vec::with_capacity(data.len() / 4 * 3 + 2);

    let mut groups = data.chunks_exact(4);
    for group in &mut groups {
        output.extend(decode_group(&config.decode_table, group)?);
    }

    let remainder = groups.remainder();
    if !remainder.is_empty() {
        let bytes = decode_group(&config.decode_table, remainder)?;
        let n_bytes = remainder.len() - 1;

        // Bits past the real data must be zero.
        // For example 2 chars == 12 bits of data,
        // which only contains 8 bits of usable data.
        // The other 4 bits are filler and should all be 0.
        if bytes[n_bytes..].iter().any(|&b| b != 0) {
            return Err(DecodeError::TrailingBits);
        }

        output.extend_from_slice(&bytes[..n_bytes]);
    }

    Ok(output)
}

fn decode_group(decode_table: &[u8; 256], group: &[u8]) -> Result<[u8; 3], DecodeError> {
    debug_assert!(group.len() <= 4);

    let mut n: u32 = 0;
    for i in 0..4 {
        let value = match group.get(i) {
            Some(&c) => match decode_table[c as usize] {
                0xFF => return Err(DecodeError::Invalid(c)),
                v => u32::from(v),
            },
            None => 0, // missing symbol: zero bits
        };
        n = (n << 6) | value;
    }

    let [_, a, b, c] = n.to_be_bytes();

    Ok([a, b, c])
}

impl Config {
    pub const fn new(alphabet: &'static [u8; 64], padding: u8) -> Result<Self, ConfigError> {
        // It is not possible to use `?` in `const fn`s.
        if let Err(err) = check_alphabet(alphabet, padding) {
            return Err(err);
        }

        let mut decode_table = [0xFF; 256];

        let mut i = 0;
        while i < alphabet.len() {
            let s = alphabet[i];
            decode_table[s as usize] = i as u8;
            i += 1;
        }

        Ok(Self {
            encode_table: alphabet,
            decode_table,
            padding,
        })
    }
}

const fn check_alphabet(alphabet: &[u8; 64], padding: u8) -> Result<(), ConfigError> {
    let mut index = 0;

    while index < 64 {
        let byte = alphabet[index];

        if byte == padding {
            return Err(ConfigError::Reserved(byte));
        }

        let mut duplicate_i = 0;
        while duplicate_i < 64 {
            if duplicate_i != index && byte == alphabet[duplicate_i] {
                return Err(ConfigError::Duplicate(byte));
            }
            duplicate_i += 1;
        }

        index += 1;
    }

    Ok(())
}

impl core::error::Error for DecodeError {}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::Invalid(c) => write!(f, "invalid input char '{c}'"),
            DecodeError::InvalidLength => write!(f, "invalid length"),
            DecodeError::InvalidPadding => write!(f, "invalid padding"),
            DecodeError::TrailingBits => write!(f, "trailing bits"),
        }
    }
}

impl core::error::Error for ConfigError {}

impl core::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ConfigError::Reserved(byte) => write!(f, "reserved byte '{byte:?}'"),
            ConfigError::Duplicate(byte) => write!(f, "duplicate byte '{byte:?}'"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_tables() {
        let config = Config::new(&ALPHABET_STANDARD, b'=').unwrap();

        assert_eq!(config.padding, b'=');
        assert_eq!(config.encode_table, ALPHABET_STANDARD);
        assert_eq!(
            config.decode_table,
            [
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 62, 255, 255, 255, 63, 52,
                53, 54, 55, 56, 57, 58, 59, 60, 61, 255, 255, 255, 255, 255, 255, 255, 0, 1, 2, 3,
                4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
                255, 255, 255, 255, 255, 255, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38,
                39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
            ]
        );

        assert_eq!(config, STANDARD);

        assert_eq!(
            Config::new(
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZZbcdefghijklmnopqrstuvwxyz0123456789+/",
                b'='
            )
            .unwrap_err(),
            ConfigError::Duplicate(b'Z'),
        );

        assert_eq!(
            Config::new(
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrs=uvwxyz0123456789+/",
                b'='
            )
            .unwrap_err(),
            ConfigError::Reserved(b'='),
        );

        assert!(Config::new(ALPHABET_STANDARD, PADDING_DEFAULT).is_ok());
        assert!(Config::new(ALPHABET_URLSAFE, PADDING_DEFAULT).is_ok());
    }

    #[test]
    fn test_decode_error() {
        assert!(!DecodeError::InvalidLength.to_string().is_empty());
        assert!(!DecodeError::InvalidPadding.to_string().is_empty());
        assert!(!DecodeError::TrailingBits.to_string().is_empty());
        assert!(!DecodeError::Invalid(b'*').to_string().is_empty());
    }

    #[test]
    fn test_config_error() {
        assert!(!ConfigError::Duplicate(b'a').to_string().is_empty());
        assert!(!ConfigError::Reserved(b'=').to_string().is_empty());
    }
}
