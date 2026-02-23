#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate libprotonup;

use libprotonup::utils::expand_tilde;

use std::path::Path;

fuzz_target!(|data: &[u8]| {
    if let Ok(utf8) = str::from_utf8(data) {
        let p = Path::new(&utf8);
        expand_tilde(p);
    }
});
