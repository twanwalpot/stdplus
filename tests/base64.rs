use stdplus::base64::{
    Config, ConfigError, DecodeError, decode, decode_urlsafe, decode_with_config, encode,
    encode_urlsafe, encode_with_config,
};

#[test]
fn test_rfc4648_vectors() {
    // https://datatracker.ietf.org/doc/html/rfc4648#section-10

    assert_eq!(&encode(""), "");
    assert_eq!(&encode("f"), "Zg==");
    assert_eq!(&encode("fo"), "Zm8=");
    assert_eq!(&encode("foo"), "Zm9v");
    assert_eq!(&encode("foob"), "Zm9vYg==");
    assert_eq!(&encode("fooba"), "Zm9vYmE=");
    assert_eq!(&encode("foobar"), "Zm9vYmFy");
}

#[test]
fn test_urlsafe_encoding() {
    assert_eq!(encode_urlsafe(b""), "");
    assert_eq!(encode_urlsafe(b"f"), "Zg==");
    assert_eq!(encode_urlsafe(b"fo"), "Zm8=");
    assert_eq!(encode_urlsafe(b"foo"), "Zm9v");

    assert_eq!(encode_urlsafe(b"\xfb"), "-w==");
    assert_eq!(encode_urlsafe(b"\xff"), "_w==");
    assert_eq!(encode_urlsafe(b"\xfb\xff"), "-_8=");
}

#[test]
fn test_urlsafe_decoding() {
    assert_eq!(decode_urlsafe(""), Ok(vec![]));
    assert_eq!(decode_urlsafe("Zg=="), Ok(b"f".to_vec()));
    assert_eq!(decode_urlsafe("Zm8="), Ok(b"fo".to_vec()));
    assert_eq!(decode_urlsafe("Zm9v"), Ok(b"foo".to_vec()));

    assert_eq!(decode_urlsafe("-w=="), Ok(vec![0xfb]));
    assert_eq!(decode_urlsafe("_w=="), Ok(vec![0xff]));
    assert_eq!(decode_urlsafe("-_8="), Ok(vec![0xfb, 0xff]));
}

#[test]
fn test_binary_data() {
    let input = [0x00, 0x01, 0x7f, 0x80, 0xfe, 0xff];
    let encoded = encode(&input);
    assert_eq!(decode(&encoded).unwrap(), input);
}

#[test]
fn test_all_byte_values() {
    let input: Vec<u8> = (0..=255).collect();
    assert_eq!(decode(&encode(&input)).unwrap(), input);
}

#[test]
fn test_padding() {
    assert_eq!(decode("Zg==").unwrap(), b"f");
    assert_eq!(decode("Zm8=").unwrap(), b"fo");
    assert_eq!(decode("Zm9v").unwrap(), b"foo");

    assert_eq!(decode("Zg").unwrap_err(), DecodeError::InvalidLength);
    assert_eq!(decode("Zm8").unwrap_err(), DecodeError::InvalidLength);
    assert_eq!(decode("Zg===").unwrap_err(), DecodeError::InvalidLength);

    assert_eq!(decode("=m9v").unwrap_err(), DecodeError::InvalidPadding);
    assert_eq!(decode("Zm=9").unwrap_err(), DecodeError::InvalidPadding);
    assert_eq!(decode("Z=8=").unwrap_err(), DecodeError::InvalidPadding);
}

#[test]
fn test_trailing_bits() {
    assert_eq!(decode("Zh==").unwrap_err(), DecodeError::TrailingBits);
    assert_eq!(decode("Zm+=").unwrap_err(), DecodeError::TrailingBits);
}

#[test]
fn test_decode_invalid_input() {
    assert_eq!(
        decode("SGVsbG8sI*HdvcmxIQ==").unwrap_err(),
        DecodeError::Invalid(b'*')
    );
    assert_eq!(decode("Zh==").unwrap_err(), DecodeError::TrailingBits);
    assert_eq!(
        decode("SGVsbG8sIHdvc").unwrap_err(),
        DecodeError::InvalidLength
    );
    assert_eq!(
        decode("SGVsbG8sIHdvcmxkIQ=").unwrap_err(),
        DecodeError::InvalidLength
    );
    assert_eq!(
        decode("SGVsbG8sIHdvcmxkIQ===").unwrap_err(),
        DecodeError::InvalidLength
    );
    assert_eq!(decode("ooo").unwrap_err(), DecodeError::InvalidLength);
    assert_eq!(decode("o===").unwrap_err(), DecodeError::InvalidPadding);
}

#[test]
fn test_custom_config() {
    let config = Config::new(
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
        b'=',
    )
    .unwrap();

    assert_eq!(
        encode_with_config(b"Hello, world!", &config),
        encode(b"Hello, world!")
    );
    assert_eq!(
        decode_with_config("SGVsbG8sIHdvcmxkIQ==", &config),
        decode("SGVsbG8sIHdvcmxkIQ==")
    );
}

#[test]
fn test_config_error() {
    assert_eq!(
        Config::new(
            b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            b'=',
        )
        .unwrap_err(),
        ConfigError::Duplicate(b'A')
    );

    assert_eq!(
        Config::new(
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+=",
            b'=',
        )
        .unwrap_err(),
        ConfigError::Reserved(b'=')
    );
}

#[test]
fn test_decode_error() {
    assert_eq!(decode("ABC*").unwrap_err(), DecodeError::Invalid(b'*'));
    assert_eq!(decode("AAAAA===").unwrap_err(), DecodeError::InvalidPadding);
    assert_eq!(decode("A").unwrap_err(), DecodeError::InvalidLength);
    assert_eq!(decode("ABCDE").unwrap_err(), DecodeError::InvalidLength);
    assert_eq!(decode("Zh==").unwrap_err(), DecodeError::TrailingBits);
}
