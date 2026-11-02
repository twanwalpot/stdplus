#![no_main]

use libfuzzer_sys::fuzz_target;
use stdplus::base64;

fuzz_target!(|data: &[u8]| {
    assert_eq!(base64::decode(&base64::encode(data)).unwrap(), data);
});
