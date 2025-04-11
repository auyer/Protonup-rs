use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct HashSums {
    pub sum_content: String,
    pub sum_type: HashSumType,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub enum HashSumType {
    Sha512,
    Sha256,
}

/// Checks the downloaded file integrity with the sha512sum
pub async fn hash_check_file<R: AsyncRead + Unpin + ?Sized>(
    file_name: &str,
    reader: &mut R,
    git_hash: HashSums,
) -> Result<bool> {
    // find line with the file name
    let expected_hash_line = git_hash
        .sum_content
        .lines()
        .find(|line| line.contains(file_name));

    let (expected_hash, _) = expected_hash_line
        // if no line found with the file name, assume the content is only the sum
        .unwrap_or(&git_hash.sum_content)
        .rsplit_once(' ')
        // if the split fails, assume the content is only the sum without any spaces
        .unwrap_or((&git_hash.sum_content, ""));

    match git_hash.sum_type {
        HashSumType::Sha512 => {
            let mut hasher = Sha512::new();
            read_all_into_digest(reader, &mut hasher)
                .await
                .context("[Hash Check] Failed reading download file for checking")?;

            let hash = hasher.finalize();

            if hex::encode(hash) != expected_hash.trim() {
                return Ok(false);
            }
            Ok(true)
        }
        HashSumType::Sha256 => {
            let mut hasher = Sha256::new();
            read_all_into_digest(reader, &mut hasher)
                .await
                .context("[Hash Check] Failed reading download file for checking")?;

            let hash = hasher.finalize();

            if hex::encode(hash) != expected_hash.trim() {
                return Ok(false);
            }
            Ok(true)
        }
    }
}

async fn read_all_into_digest<R: AsyncRead + Unpin + ?Sized, D: Digest>(
    read: &mut R,
    digest: &mut D,
) -> Result<()> {
    const BUFFER_LEN: usize = 8 * 1024; // 8KB
    let mut buffer = [0u8; BUFFER_LEN];

    loop {
        let count = read.read(&mut buffer).await?;
        digest.update(&buffer[..count]);
        if count != BUFFER_LEN {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use sha2::{Digest, Sha256, Sha512};

    use crate::hashing::{HashSumType, HashSums};

    #[tokio::test]
    async fn hash_check_file() {
        let test_data = b"This Is A Test";
        let hash = hex::encode(Sha512::new_with_prefix(test_data).finalize());

        assert!(
            super::hash_check_file(
                "",
                &mut &test_data[..],
                HashSums {
                    sum_content: hash,
                    sum_type: HashSumType::Sha512
                }
            )
            .await
            .unwrap(),
            "Hash didn't match"
        );

        let hash = hex::encode(Sha256::new_with_prefix(test_data).finalize());

        assert!(
            super::hash_check_file(
                "",
                &mut &test_data[..],
                HashSums {
                    sum_content: hash,
                    sum_type: HashSumType::Sha256
                }
            )
            .await
            .unwrap(),
            "Hash didn't match"
        );
    }
}
