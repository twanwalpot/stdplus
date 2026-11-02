#![no_main]

use base64::prelude::*;
use libfuzzer_sys::fuzz_target;
use stdplus::base64 as stdplus_base64;

fuzz_target!(|data: &[u8]| {
    let ours = stdplus_base64::encode(data);
    let oracle = BASE64_STANDARD.encode(data);
    assert_eq!(ours, oracle, "input={data:?}")
});
