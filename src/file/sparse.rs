use anyhow::{Context, Result};
use rustix::{
    fs::{SeekFrom, seek},
    io::Errno,
};
use std::{
    fs::File,
    io::{self, Read, Seek},
    ops::Range,
    path::Path,
};

/// Returns the byte ranges of `file` between `start` and `end` that hold data.
pub fn data_ranges(file: &File, start: u64, end: u64) -> Result<Vec<Range<u64>>> {
    let mut ranges = vec![];
    let mut pos = start;
    while pos < end {
        let data = match seek(file, SeekFrom::Data(pos)) {
            Ok(data) => data,
            // no data after pos
            Err(Errno::NXIO) => break,
            Err(e) => return Err(e).context("data_ranges: cannot find data"),
        };
        if data >= end {
            break;
        }
        let hole = seek(file, SeekFrom::Hole(data)).context("data_ranges: cannot find hole")?;
        ranges.push(data..hole.min(end));
        pos = hole;
    }

    Ok(ranges)
}

/// Sorts `ranges` and joins the ones that overlap or touch.
pub fn merge(mut ranges: Vec<Range<u64>>) -> Vec<Range<u64>> {
    ranges.sort_by_key(|r| r.start);
    let mut merged: Vec<Range<u64>> = Vec::with_capacity(ranges.len());
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    merged
}

/// Copies `len` bytes of `src` at `src_offset` to `dst` at `dst_offset`. Holes of `src` are
/// skipped, so they stay holes in a new `dst`.
fn copy_range(src: &File, src_offset: u64, dst: &File, dst_offset: u64, len: u64) -> Result<()> {
    let src_len = src
        .metadata()
        .context("copy_range: cannot get source size")?
        .len();
    anyhow::ensure!(
        src_offset + len <= src_len,
        "copy_range: source has {src_len} bytes, cannot copy {len} bytes at {src_offset}"
    );

    for data in data_ranges(src, src_offset, src_offset + len)? {
        let mut src = src;
        let mut dst = dst;
        src.seek(io::SeekFrom::Start(data.start))
            .context("copy_range: cannot seek source")?;
        dst.seek(io::SeekFrom::Start(data.start - src_offset + dst_offset))
            .context("copy_range: cannot seek destination")?;
        // one copy_file_range call copies at most 2 GiB; io::copy repeats it until a range is done
        let copied = io::copy(&mut src.take(data.end - data.start), &mut dst)
            .context("copy_range: cannot copy data")?;
        anyhow::ensure!(
            copied == data.end - data.start,
            "copy_range: source ended early"
        );
    }

    Ok(())
}

/// Creates `dst` with `len` bytes and copies `len` bytes of `src` at `src_offset` into it.
/// Holes of `src` stay holes in `dst`.
pub fn copy_to_new_file(src: &File, src_offset: u64, dst: &Path, len: u64) -> Result<()> {
    let dst =
        File::create(dst).context(format!("copy_to_new_file: cannot create {}", dst.display()))?;

    copy_to_empty_file(src, src_offset, &dst, len)
}

/// Sets the size of the empty `dst` to `len` and copies `len` bytes of `src` at `src_offset`
/// into it. Holes of `src` stay holes in `dst`.
fn copy_to_empty_file(src: &File, src_offset: u64, dst: &File, len: u64) -> Result<()> {
    dst.set_len(len)
        .context("copy_to_empty_file: cannot set destination size")?;

    copy_range(src, src_offset, dst, 0, len)
}

/// Copies the file `src` into the empty `dst`. Holes of `src` stay holes in `dst`.
pub fn copy_file(src: &Path, dst: &File) -> Result<()> {
    let src_file = File::open(src).context(format!("copy_file: cannot open {}", src.display()))?;
    let len = src_file
        .metadata()
        .context(format!(
            "copy_file: cannot get the size of {}",
            src.display()
        ))?
        .len();

    copy_to_empty_file(&src_file, 0, dst, len)
}

#[cfg(test)]
mod tests {
    use crate::file::sparse::*;
    use std::os::unix::fs::FileExt;

    const BLOCK_SIZE: u64 = 4096;
    const FILE_BLOCKS: u64 = 8;
    const DATA_BLOCK: u64 = 2;
    const DATA_BYTE: u8 = 0x5a;
    const DST_OFFSET_BLOCKS: u64 = 1;

    /// A sparse file with data in `DATA_BLOCK` and holes everywhere else.
    fn sparse_file(dir: &std::path::Path) -> File {
        let file = File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(dir.join("src"))
            .expect("create file");
        file.set_len(FILE_BLOCKS * BLOCK_SIZE).expect("set size");
        file.write_all_at(&[DATA_BYTE; BLOCK_SIZE as usize], DATA_BLOCK * BLOCK_SIZE)
            .expect("write data block");
        file
    }

    #[test]
    fn data_ranges_finds_data_between_holes() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let file = sparse_file(dir.path());
        let data = DATA_BLOCK * BLOCK_SIZE..(DATA_BLOCK + 1) * BLOCK_SIZE;

        assert_eq!(
            data_ranges(&file, 0, FILE_BLOCKS * BLOCK_SIZE).expect("data ranges"),
            vec![data.clone()]
        );
        assert_eq!(
            data_ranges(&file, 0, data.start).expect("data ranges"),
            vec![]
        );
        assert_eq!(
            data_ranges(&file, data.start + 1, data.end - 1).expect("data ranges"),
            vec![data.start + 1..data.end - 1]
        );
    }

    #[test]
    fn merge_joins_overlapping_and_touching_ranges() {
        assert_eq!(
            merge(vec![8..10, 0..2, 1..4, 4..5, 12..12]),
            vec![0..5, 8..10, 12..12]
        );
    }

    #[test]
    fn copy_to_new_file_keeps_holes() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let src = sparse_file(dir.path());
        let dst_path = dir.path().join("dst");
        let len = (FILE_BLOCKS - DST_OFFSET_BLOCKS) * BLOCK_SIZE;

        copy_to_new_file(&src, DST_OFFSET_BLOCKS * BLOCK_SIZE, &dst_path, len)
            .expect("copy to new file");

        let dst = File::open(&dst_path).expect("open file");
        assert_eq!(dst.metadata().expect("metadata").len(), len);
        let data_start = (DATA_BLOCK - DST_OFFSET_BLOCKS) * BLOCK_SIZE;
        assert_eq!(
            data_ranges(&dst, 0, len).expect("data ranges"),
            vec![data_start..data_start + BLOCK_SIZE]
        );
        let mut block = [0u8; BLOCK_SIZE as usize];
        dst.read_exact_at(&mut block, data_start)
            .expect("read block");
        assert!(block.iter().all(|b| *b == DATA_BYTE));
    }

    #[test]
    fn copy_file_keeps_holes() {
        let dir = tempfile::tempdir().expect("create temp dir");
        sparse_file(dir.path());
        let dst = File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(dir.path().join("dst"))
            .expect("create file");

        copy_file(&dir.path().join("src"), &dst).expect("copy file");

        let len = FILE_BLOCKS * BLOCK_SIZE;
        assert_eq!(dst.metadata().expect("metadata").len(), len);
        assert_eq!(
            data_ranges(&dst, 0, len).expect("data ranges"),
            vec![DATA_BLOCK * BLOCK_SIZE..(DATA_BLOCK + 1) * BLOCK_SIZE]
        );
    }

    #[test]
    fn copy_range_rejects_range_after_source_end() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let src = sparse_file(dir.path());
        let dst = File::create(dir.path().join("dst")).expect("create file");

        assert!(copy_range(&src, BLOCK_SIZE, &dst, 0, FILE_BLOCKS * BLOCK_SIZE).is_err());
    }
}
