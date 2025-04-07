#![no_main]

use std::str::FromStr;

use libfuzzer_sys::fuzz_target;

extern crate libprotonup;

use libprotonup::{
    sources::{Forge, Source},
};

fuzz_target!(|data: &[u8]| {
    if let Ok(utf8) = str::from_utf8(data) {
        let utf8 = utf8.to_owned();
        let s = Source::new_custom(
            utf8.clone(),
            Forge::GitHub,
            utf8.clone(),
            utf8.clone(),
            Some((utf8.clone(), utf8.clone())),
            Some(utf8.clone()),
        );

        assert!(format!("{s}") == utf8);
    }
});
