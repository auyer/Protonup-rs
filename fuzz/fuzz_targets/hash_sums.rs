#![no_main]

use libfuzzer_sys::fuzz_target;
use sha2::{Digest, Sha256, Sha512};
use std::sync::OnceLock;
use tokio::runtime::Runtime;

extern crate libprotonup;

use libprotonup::hashing::{HashSumType, HashSums, hash_check_file};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fuzz_target!(|data: &[u8]| {
    // Layout: [file name][0x00][checksum file contents][0x00][payload bytes]
    // The first two fields are user/network text; the payload stays raw.
    let mut fields = data.splitn(3, |byte| *byte == 0);
    let file_name = String::from_utf8_lossy(fields.next().unwrap_or_default()).into_owned();
    let sum_content = String::from_utf8_lossy(fields.next().unwrap_or_default()).into_owned();
    let payload = fields.next().unwrap_or_default();

    let rt = RUNTIME.get_or_init(|| Runtime::new().expect("failed to create tokio runtime"));

    rt.block_on(async {
        // Arbitrary checksum-file contents must not panic for either hash type.
        for sum_type in [HashSumType::Sha512, HashSumType::Sha256] {
            let mut reader = payload;
            let _ = hash_check_file(
                &file_name,
                &mut reader,
                HashSums {
                    sum_content: sum_content.clone(),
                    sum_type,
                },
            )
            .await;
        }

        // The pre-existing happy path stays intact: a checksum file containing
        // only the correct digest of the payload verifies successfully, even
        // when the file name is arbitrary/non-ASCII.
        let sha512 = hex::encode(Sha512::new_with_prefix(payload).finalize());
        let sha256 = hex::encode(Sha256::new_with_prefix(payload).finalize());

        for (sum_content, sum_type) in
            [(sha512, HashSumType::Sha512), (sha256, HashSumType::Sha256)]
        {
            let mut reader = payload;
            let result = hash_check_file(
                &file_name,
                &mut reader,
                HashSums {
                    sum_content,
                    sum_type,
                },
            )
            .await;
            assert!(
                result.is_ok(),
                "in-memory hash check failed unexpectedly: {result:?}"
            );
            assert!(
                result.unwrap(),
                "correct digest was not accepted for file name {file_name:?}"
            );
        }
    });
});
