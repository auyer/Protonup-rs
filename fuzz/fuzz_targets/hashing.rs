#![no_main]

use libfuzzer_sys::fuzz_target;
use sha2::{Digest, Sha256, Sha512};
use std::io::Cursor;
use tokio::runtime::Runtime;

extern crate libprotonup;

use libprotonup::hashing::{HashSumType, HashSums, hash_check_file};

fuzz_target!(|data: &[u8]| {
    let test_data = data.to_vec();
    let hash512 = hex::encode(Sha512::new_with_prefix(&test_data).finalize());
    let hash256 = hex::encode(Sha256::new_with_prefix(&test_data).finalize());

    // Create a Tokio runtime to execute async code
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        let mut test_data_cursor = Cursor::new(test_data.clone());
        let mut test_data_cursor2 = Cursor::new(test_data.clone());

        // SHA512
        let check512 = hash_check_file(
            "",
            &mut test_data_cursor,
            HashSums {
                sum_content: hash512,
                sum_type: HashSumType::Sha512,
            },
        );
        // SHA256
        let check256 = hash_check_file(
            "",
            &mut test_data_cursor2,
            HashSums {
                sum_content: hash256,
                sum_type: HashSumType::Sha256,
            },
        );

        let (check512, check256) = tokio::join!(check512, check256);

        assert!(check512.is_ok());
        assert!(check512.unwrap());

        assert!(check256.is_ok());
        assert!(check256.unwrap());
    });
});
