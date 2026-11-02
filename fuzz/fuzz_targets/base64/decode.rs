#![no_main]

use base64::prelude::*;
use libfuzzer_sys::fuzz_target;
use stdplus::base64 as stdplus_base64;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let ours = stdplus_base64::decode(input);
    let oracle = BASE64_STANDARD.decode(input.as_bytes());

    match (ours, oracle) {
        (Ok(a), Ok(b)) => assert_eq!(a, b, "input={input:?}"),
        (Err(_), Err(_)) => {}
        (Ok(a), Err(e)) => panic!("we accepted, oracle rejected ({e}): input={input:?} -> {a:?}"),
        (Err(e), Ok(b)) => panic!("we rejected ({e:?}), oracle accepted: input={input:?} -> {b:?}"),
    }
});
