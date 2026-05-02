use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};
use std::fs::{File, create_dir_all, rename};
use std::io::{BufRead, BufReader, BufWriter, Lines, Write};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

pub fn write_jsonl<T: Serialize>(path: impl AsRef<Path>, rows: &[T]) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent)?;
    }

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    for row in rows {
        let line = serde_json::to_string(row)?;
        writer.write_all(line.as_bytes())?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;
    Ok(())
}

pub fn write_one_json<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent)?;
    }

    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, value)?;
    Ok(())
}

pub fn read_jsonl_batches<T: DeserializeOwned>(
    path: impl AsRef<Path>,
    batch_size: usize,
) -> Result<JsonlBatchReader<T>> {
    let path = path.as_ref().to_path_buf();
    let file = File::open(&path)?;
    Ok(JsonlBatchReader {
        lines: BufReader::new(file).lines(),
        path,
        batch_size: batch_size.max(1),
        line_num: 0,
        finished: false,
        _marker: PhantomData,
    })
}

pub fn read_jsonl_strict<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<Vec<T>> {
    let mut items = Vec::new();
    for batch in read_jsonl_batches(path, 1000)? {
        items.extend(batch?);
    }
    Ok(items)
}

pub struct JsonlBatchReader<T> {
    lines: Lines<BufReader<File>>,
    path: PathBuf,
    batch_size: usize,
    line_num: usize,
    finished: bool,
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned> Iterator for JsonlBatchReader<T> {
    type Item = Result<Vec<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let mut batch = Vec::with_capacity(self.batch_size);
        while batch.len() < self.batch_size {
            match self.lines.next() {
                Some(Ok(line)) => {
                    self.line_num += 1;
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    match serde_json::from_str(line) {
                        Ok(item) => batch.push(item),
                        Err(e) => {
                            self.finished = true;
                            return Some(Err(anyhow::anyhow!(
                                "Strict JSONL parsing failed in {} at line {}: {}",
                                self.path.display(),
                                self.line_num,
                                e
                            )));
                        }
                    }
                }
                Some(Err(e)) => {
                    self.finished = true;
                    return Some(Err(e.into()));
                }
                None => {
                    self.finished = true;
                    break;
                }
            }
        }

        if batch.is_empty() {
            None
        } else {
            Some(Ok(batch))
        }
    }
}

/// Atomically writes JSONL data to a file using a temporary file.
/// This ensures the original file is only replaced when the write is complete,
/// preventing data loss if the write fails mid-operation.
///
/// # Arguments
/// * `path` - Target file path
/// * `rows` - Data to write
///
/// # Returns
/// Ok(()) if successful, or an error if the write fails
pub fn write_jsonl_atomic<T: Serialize>(path: impl AsRef<Path>, rows: &[T]) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }

    // Write to a unique temporary file in the same directory
    let uuid = uuid::Uuid::new_v4();
    let temp_path = path.with_file_name(format!(
        "{}.{}.tmp",
        path.file_name().unwrap().to_str().unwrap(),
        uuid
    ));
    let file = File::create(&temp_path)?;
    let mut writer = BufWriter::new(file);

    for row in rows {
        let line = serde_json::to_string(row)?;
        writer.write_all(line.as_bytes())?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;
    drop(writer);

    // Atomically replace the original file with the temporary file
    if path.exists() {
        rename(&temp_path, path)?;
    } else {
        rename(&temp_path, path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::read_jsonl_strict;
    use serde::{Deserialize, Serialize};
    use std::fs;

    #[derive(Debug, Serialize, Deserialize)]
    struct Row {
        id: String,
    }

    #[test]
    fn strict_jsonl_fails_on_malformed_line() {
        let path = std::env::temp_dir().join(format!(
            "orsgraph-strict-jsonl-{}.jsonl",
            uuid::Uuid::new_v4()
        ));
        fs::write(&path, "{\"id\":\"ok\"}\n{bad json}\n").expect("write jsonl");

        let err = read_jsonl_strict::<Row>(&path).expect_err("strict parse should fail");
        let message = err.to_string();
        let _ = fs::remove_file(path);

        assert!(message.contains("Strict JSONL parsing failed"));
        assert!(message.contains("line 2"));
    }

    #[test]
    fn strict_jsonl_reads_valid_lines() {
        let path = std::env::temp_dir().join(format!(
            "orsgraph-strict-jsonl-{}.jsonl",
            uuid::Uuid::new_v4()
        ));
        fs::write(&path, "{\"id\":\"a\"}\n\n{\"id\":\"b\"}\n").expect("write jsonl");

        let rows = super::read_jsonl_strict::<Row>(&path).expect("strict parse");
        let _ = fs::remove_file(path);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "a");
        assert_eq!(rows[1].id, "b");
    }

    #[test]
    fn test_write_read_jsonl() {
        let path = std::env::temp_dir().join(format!("test-{}.jsonl", uuid::Uuid::new_v4()));
        let rows = vec![
            Row {
                id: "1".to_string(),
            },
            Row {
                id: "2".to_string(),
            },
        ];
        super::write_jsonl(&path, &rows).unwrap();
        let read_rows = super::read_jsonl_strict::<Row>(&path).unwrap();
        let _ = fs::remove_file(path);
        assert_eq!(read_rows.len(), 2);
    }

    #[test]
    fn test_write_one_json() {
        let path = std::env::temp_dir().join(format!("test-{}.json", uuid::Uuid::new_v4()));
        let row = Row {
            id: "1".to_string(),
        };
        super::write_one_json(&path, &row).unwrap();
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_write_jsonl_atomic() {
        let path = std::env::temp_dir().join(format!("test-atomic-{}.jsonl", uuid::Uuid::new_v4()));
        let rows = vec![Row {
            id: "1".to_string(),
        }];
        super::write_jsonl_atomic(&path, &rows).unwrap();
        assert!(path.exists());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_jsonl_read_large_buffer() {
        let path = std::env::temp_dir().join(format!("test-large-{}.jsonl", uuid::Uuid::new_v4()));
        let mut rows = Vec::new();
        for i in 0..5000 {
            rows.push(Row {
                id: format!("id_{:05}", i),
            });
        }
        super::write_jsonl(&path, &rows).unwrap();

        let mut count = 0;
        for batch in super::read_jsonl_batches::<Row>(&path, 1000).unwrap() {
            let batch = batch.unwrap();
            count += batch.len();
        }
        let _ = fs::remove_file(path);
        assert_eq!(count, 5000);
    }
}
