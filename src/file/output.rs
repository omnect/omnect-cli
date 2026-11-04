use crate::file::{bmap, compression::Compression, sparse};
use anyhow::{Context, Result};
use log::{info, warn};
use rustix::{
    fs::{Access, AtFlags, CWD, StatxAttributes, StatxFlags},
    io::Errno,
};
use std::{
    collections::BTreeSet,
    env,
    fs::{self, File, OpenOptions},
    io::ErrorKind,
    iter,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

const TMP_DIR_PREFIX: &str = ".omnect-cli-";

/// Returns `explicit`, else `image` if it has an extension of `target`, else `image` without the
/// extension of its `source` compression, with the extension of `target` appended. Fails if
/// `explicit` has no extension of `target`, or a compression extension while `target` is none.
pub fn output_image(
    image: &Path,
    source: Option<&Compression>,
    target: Option<&Compression>,
    explicit: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(path) = explicit {
        match target {
            Some(c) => anyhow::ensure!(
                c.matches_extension(path),
                "{} has no extension of the -p compression; name it e.g. {}",
                path.display(),
                c.compressed_path(path).display()
            ),
            None => anyhow::ensure!(
                !Compression::has_extension(path),
                "{} has a compression extension, but the image is written uncompressed; pass -p",
                path.display()
            ),
        }
        return Ok(path.to_path_buf());
    }
    if target.is_some_and(|c| c.matches_extension(image)) {
        return Ok(image.to_path_buf());
    }

    let mut output = image.to_path_buf();
    if source.is_some_and(|c| c.matches_extension(image)) {
        output.set_extension("");
    }
    Ok(match target {
        Some(c) => c.compressed_path(&output),
        None => output,
    })
}

/// Fails if writing `output_image` and its bmap would overwrite a file the user did not name,
/// replace the bmap of an image that stays, or leave a bmap that `bmaptool` would use for the
/// output image and that does not match it.
pub fn check_conflicts(
    input_image: &Path,
    output_image: &Path,
    explicit_output: bool,
    output_bmap: bool,
    input_bmap: Option<&Path>,
) -> Result<()> {
    let in_place = same_file(input_image, output_image)?;
    ensure_writable(output_image)?;

    anyhow::ensure!(
        explicit_output || in_place || !output_image.exists(),
        "{} exists; choose the output image with --output-image",
        output_image.display()
    );
    if output_bmap {
        let target_bmap = bmap::bmap_path(output_image);
        ensure_writable(&target_bmap)?;
        if !in_place {
            let found_bmap = bmap::find_input_bmap(input_image, None)?;
            for bmap in input_bmap.into_iter().chain(found_bmap.as_deref()) {
                anyhow::ensure!(
                    !same_file(bmap, &target_bmap)?,
                    "{} is the bmap of {} and would be replaced; choose another output image \
                     with --output-image",
                    bmap.display(),
                    input_image.display()
                );
            }
        }
    } else if let Some(stale_bmap) = bmap::find_input_bmap(output_image, None)? {
        anyhow::bail!(
            "{} would not match {}; pass -b to write a matching bmap",
            stale_bmap.display(),
            output_image.display()
        );
    }

    Ok(())
}

/// Fails if `path` exists and the user cannot write it, so a read-only file is not replaced.
fn ensure_writable(path: &Path) -> Result<()> {
    match rustix::fs::access(path, Access::WRITE_OK) {
        Err(Errno::NOENT) => Ok(()),
        result => result.context(format!(
            "cannot write {}; make it writable or choose another output image with \
             --output-image",
            path.display()
        )),
    }
}

/// A new file in the directory of its destination, which `commit` renames to the destination.
/// Until then the destination stays unchanged. A rename cannot replace a mount point, e.g. an
/// image file mounted into a container, so for such a destination the new file is staged in
/// TMPDIR and copied into it.
pub struct Staged {
    tmp_dir: TempDir,
    file: PathBuf,
    dest: PathBuf,
    mount_root: bool,
}

impl Staged {
    /// Writes and syncs the new file for `dest`. A symlink is followed, and an existing `dest`
    /// keeps its permissions.
    pub fn write(dest: &Path, write: impl FnOnce(&mut File) -> Result<()>) -> Result<Self> {
        let dest = resolve(dest)?;
        let dir = dest
            .parent()
            .context(format!("{} has no directory", dest.display()))?;
        let mount_root = is_mount_root(&dest);
        let tmp_parent = if mount_root {
            env::temp_dir()
        } else {
            dir.to_path_buf()
        };
        let tmp_dir = tempfile::Builder::new()
            .prefix(TMP_DIR_PREFIX)
            .tempdir_in(&tmp_parent)
            .context(format!(
                "cannot create a temp dir in {}",
                tmp_parent.display()
            ))?;
        let file = tmp_dir.path().join(
            dest.file_name()
                .context(format!("{} has no file name", dest.display()))?,
        );

        let mut handle =
            File::create(&file).context(format!("cannot create {}", file.display()))?;
        write(&mut handle)?;
        if let Ok(metadata) = fs::metadata(&dest) {
            handle
                .set_permissions(metadata.permissions())
                .context(format!("cannot set permissions of {}", file.display()))?;
        }
        // a file in TMPDIR is only copied into `dest`, and the copy is synced
        if !mount_root {
            // the handle that wrote the data also gets the write errors
            handle
                .sync_all()
                .context(format!("cannot sync {}", file.display()))?;
        }

        Ok(Self {
            tmp_dir,
            file,
            dest,
            mount_root,
        })
    }

    fn rename(&mut self) -> Result<()> {
        if self.mount_root {
            return self.copy_into_dest();
        }
        match fs::rename(&self.file, &self.dest) {
            Err(e)
                if matches!(
                    e.kind(),
                    ErrorKind::ResourceBusy | ErrorKind::CrossesDevices
                ) =>
            {
                info!(
                    "cannot rename to {}: {e}; copying into it instead",
                    self.dest.display()
                );
                self.copy_into_dest()
            }
            result => result.context(format!(
                "cannot rename {} to {}",
                self.file.display(),
                self.dest.display()
            )),
        }
    }

    /// Keeps the staged file if the copy fails, because `dest` is truncated already.
    fn copy_into_dest(&mut self) -> Result<()> {
        let dest = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.dest)
            .context(format!("cannot open {}", self.dest.display()))?;

        let result = sparse::copy_file(&self.file, &dest).and_then(|()| {
            dest.sync_all()
                .context(format!("cannot sync {}", self.dest.display()))
        });
        if result.is_err() {
            self.tmp_dir.disable_cleanup(true);
        }
        result.with_context(|| {
            format!(
                "cannot copy into {}; the new file is kept in {}",
                self.dest.display(),
                self.file.display()
            )
        })
    }
}

/// `false` if the kernel cannot tell.
fn is_mount_root(path: &Path) -> bool {
    rustix::fs::statx(
        CWD,
        path,
        AtFlags::SYMLINK_NOFOLLOW | AtFlags::NO_AUTOMOUNT,
        StatxFlags::empty(),
    )
    .is_ok_and(|statx| {
        statx
            .stx_attributes_mask
            .contains(StatxAttributes::MOUNT_ROOT)
            && statx.stx_attributes.contains(StatxAttributes::MOUNT_ROOT)
    })
}

/// Renames the staged `image` and its `bmap`, then syncs their directories, so the renames are
/// not lost on a power loss. The old bmap is removed before the image is renamed, so if renaming
/// the bmap fails, no bmap is left that does not match the new image.
pub fn commit(mut image: Staged, mut bmap: Option<Staged>) -> Result<()> {
    if let Some(bmap) = bmap.as_ref().filter(|bmap| !bmap.mount_root) {
        // a mount point that was not detected cannot be removed, the copy into it replaces it
        match fs::remove_file(&bmap.dest) {
            Err(e) if !matches!(e.kind(), ErrorKind::NotFound | ErrorKind::ResourceBusy) => {
                return Err(e).context(format!("cannot remove {}", bmap.dest.display()));
            }
            _ => {}
        }
    }
    image.rename()?;
    if let Some(bmap) = &mut bmap {
        bmap.rename()?;
    }

    let dirs: BTreeSet<_> = iter::once(&image)
        .chain(&bmap)
        .filter_map(|staged| staged.dest.parent())
        .collect();
    for dir in dirs {
        // both files are complete already, only a power loss could undo the rename
        if let Err(e) = File::open(dir).and_then(|dir| dir.sync_all()) {
            warn!("cannot sync {}: {e}", dir.display());
        }
    }

    Ok(())
}

fn same_file(a: &Path, b: &Path) -> Result<bool> {
    Ok(resolve(a)? == resolve(b)?)
}

/// The absolute path of `path` with symlinks resolved. A file that does not exist yet is
/// resolved through its directory.
fn resolve(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return fs::canonicalize(path).context(format!("cannot resolve {}", path.display()));
    }
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    let name = path
        .file_name()
        .context(format!("{} has no file name", path.display()))?;

    Ok(fs::canonicalize(dir)
        .context(format!("cannot resolve {}", dir.display()))?
        .join(name))
}

#[cfg(test)]
mod tests {
    use crate::file::output::*;
    use std::{
        io::Write,
        os::unix::fs::{PermissionsExt, symlink},
    };

    const IMAGE: &str = "image.wic";
    const IMAGE_XZ: &str = "image.wic.xz";
    const OTHER_IMAGE: &str = "device.wic";
    const XZ_LEVEL: u32 = 9;
    const OLD_CONTENT: &[u8] = b"old image";
    const NEW_CONTENT: &[u8] = b"new image";
    const IMAGE_MODE: u32 = 0o644;
    const READ_ONLY_MODE: u32 = 0o444;
    const MODE_MASK: u32 = 0o777;
    const UNRESIZABLE_FILE: &str = "/dev/full";

    fn touch(path: &Path) {
        fs::write(path, []).expect("create file");
    }

    #[test]
    fn output_image_follows_compression() {
        let image = Path::new(IMAGE);
        let image_xz = Path::new(IMAGE_XZ);
        let xz = Compression::xz {
            compression_level: XZ_LEVEL,
        };
        let output = |image, source, target| {
            output_image(image, source, target, None).expect("output image")
        };

        assert_eq!(output(image, None, None), image);
        assert_eq!(output(image_xz, Some(&xz), None), image);
        assert_eq!(output(image, None, Some(&xz)), image_xz);
        assert_eq!(output(image_xz, Some(&xz), Some(&xz)), image_xz);
    }

    #[test]
    fn output_image_keeps_every_extension_of_the_same_compression() {
        for (image, compression) in [
            ("image.wic.gz", Compression::gzip),
            ("image.wic.gzip", Compression::gzip),
            ("image.wic.bz2", Compression::bzip2),
            ("image.wic.bzip2", Compression::bzip2),
        ] {
            let image = Path::new(image);
            assert_eq!(
                output_image(image, Some(&compression), Some(&compression), None)
                    .expect("output image"),
                image
            );
        }
    }

    #[test]
    fn output_image_strips_only_the_source_compression_extension() {
        let image = Path::new("image.bin");
        let xz = Compression::xz {
            compression_level: XZ_LEVEL,
        };

        assert_eq!(
            output_image(image, Some(&xz), None, None).expect("output image"),
            image
        );
    }

    #[test]
    fn output_image_takes_explicit_path_with_matching_extension() {
        let xz = Compression::xz {
            compression_level: XZ_LEVEL,
        };
        let other_xz = Path::new("device.wic.xz");

        assert_eq!(
            output_image(
                Path::new(IMAGE_XZ),
                Some(&xz),
                None,
                Some(Path::new(OTHER_IMAGE))
            )
            .expect("uncompressed"),
            Path::new(OTHER_IMAGE)
        );
        assert_eq!(
            output_image(Path::new(IMAGE), None, Some(&xz), Some(other_xz)).expect("compressed"),
            other_xz
        );
    }

    #[test]
    fn output_image_rejects_explicit_path_with_wrong_extension() {
        let xz = Compression::xz {
            compression_level: XZ_LEVEL,
        };

        assert!(
            output_image(
                Path::new(IMAGE),
                None,
                Some(&xz),
                Some(Path::new(OTHER_IMAGE))
            )
            .is_err()
        );
        assert!(
            output_image(
                Path::new(IMAGE),
                None,
                Some(&xz),
                Some(Path::new("device.wic.gz"))
            )
            .is_err()
        );
        assert!(
            output_image(
                Path::new(IMAGE_XZ),
                Some(&xz),
                None,
                Some(Path::new("device.wic.xz"))
            )
            .is_err()
        );
    }

    #[test]
    fn check_conflicts_allows_in_place_with_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        touch(&image);
        touch(&image_bmap);

        check_conflicts(&image, &image, false, true, Some(&image_bmap)).expect("no conflict");
    }

    #[test]
    fn check_conflicts_rejects_a_read_only_output() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        touch(&image);
        touch(&image_bmap);
        let read_only = fs::Permissions::from_mode(READ_ONLY_MODE);
        fs::set_permissions(&image_bmap, read_only.clone()).expect("set permissions");
        // root can write a read-only file
        if OpenOptions::new().write(true).open(&image_bmap).is_ok() {
            return;
        }

        assert!(check_conflicts(&image, &image, false, true, Some(&image_bmap)).is_err());
        fs::remove_file(&image_bmap).expect("remove bmap");
        check_conflicts(&image, &image, false, false, None).expect("writable image");
        fs::set_permissions(&image, read_only).expect("set permissions");
        assert!(check_conflicts(&image, &image, false, false, None).is_err());
    }

    #[test]
    fn check_conflicts_rejects_existing_derived_image() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image = dir.path().join(IMAGE);
        touch(&image_xz);
        touch(&image);

        assert!(check_conflicts(&image_xz, &image, false, false, None).is_err());
        check_conflicts(&image_xz, &image, true, false, None).expect("explicit output");
    }

    #[test]
    fn check_conflicts_rejects_replacing_the_input_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        touch(&image_xz);
        touch(&image_bmap);

        assert!(check_conflicts(&image_xz, &image, false, true, Some(&image_bmap)).is_err());
        let other = dir.path().join(OTHER_IMAGE);
        check_conflicts(&image_xz, &other, true, true, Some(&image_bmap)).expect("other output");
    }

    #[test]
    fn check_conflicts_rejects_leaving_a_stale_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        touch(&image);
        touch(&bmap::bmap_path(&image));

        assert!(check_conflicts(&image, &image, false, false, None).is_err());
    }

    #[test]
    fn check_conflicts_rejects_a_stale_bmap_bmaptool_finds() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        touch(&image);
        touch(&dir.path().join("image.bmap"));

        assert!(check_conflicts(&image, &image, false, false, None).is_err());
    }

    #[test]
    fn check_conflicts_rejects_a_stale_bmap_of_the_uncompressed_image() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image_bmap = bmap::bmap_path(&dir.path().join(IMAGE));
        touch(&image_xz);
        touch(&image_bmap);

        // bmaptool uses image.wic.bmap for image.wic.xz
        assert!(check_conflicts(&image_xz, &image_xz, false, false, None).is_err());
        check_conflicts(&image_xz, &image_xz, false, true, Some(&image_bmap)).expect("with -b");
    }

    #[test]
    fn check_conflicts_rejects_replacing_the_bmap_next_to_the_input_image() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image = dir.path().join(IMAGE);
        let other_bmap = dir.path().join("other.bmap");
        touch(&image_xz);
        touch(&bmap::bmap_path(&image));
        touch(&other_bmap);

        // --input-bmap and --no-input-bmap
        assert!(check_conflicts(&image_xz, &image, false, true, Some(&other_bmap)).is_err());
        assert!(check_conflicts(&image_xz, &image, false, true, None).is_err());
    }

    fn stage(dest: &Path, content: &'static [u8]) -> Staged {
        Staged::write(dest, |file| Ok(file.write_all(content)?)).expect("stage")
    }

    #[test]
    fn staged_write_keeps_dest_when_write_fails() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        fs::write(&image, OLD_CONTENT).expect("write image");

        assert!(
            Staged::write(&image, |file| {
                file.write_all(NEW_CONTENT).expect("write file");
                anyhow::bail!("write failed")
            })
            .is_err()
        );
        assert_eq!(fs::read(&image).expect("read image"), OLD_CONTENT);
        assert_eq!(
            fs::read_dir(dir.path()).expect("read dir").count(),
            1,
            "temp dir was left"
        );
    }

    #[test]
    fn staged_commit_keeps_permissions_and_symlink() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let link = dir.path().join(OTHER_IMAGE);
        fs::write(&image, OLD_CONTENT).expect("write image");
        fs::set_permissions(&image, fs::Permissions::from_mode(IMAGE_MODE))
            .expect("set permissions");
        symlink(&image, &link).expect("create symlink");

        commit(stage(&link, NEW_CONTENT), None).expect("commit");

        assert!(link.is_symlink());
        assert_eq!(fs::read(&image).expect("read image"), NEW_CONTENT);
        assert_eq!(
            fs::metadata(&image).expect("metadata").permissions().mode() & MODE_MASK,
            IMAGE_MODE
        );
    }

    #[test]
    fn commit_leaves_no_old_bmap_when_the_bmap_rename_fails() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        fs::write(&image, OLD_CONTENT).expect("write image");
        fs::write(&image_bmap, OLD_CONTENT).expect("write bmap");

        let staged_bmap = stage(&image_bmap, NEW_CONTENT);
        fs::remove_file(&staged_bmap.file).expect("remove staged bmap");

        assert!(commit(stage(&image, NEW_CONTENT), Some(staged_bmap)).is_err());
        assert_eq!(fs::read(&image).expect("read image"), NEW_CONTENT);
        assert!(!image_bmap.exists());
    }

    #[test]
    fn copy_into_dest_replaces_the_content() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        fs::write(&image, b"old image with more bytes").expect("write image");

        stage(&image, NEW_CONTENT)
            .copy_into_dest()
            .expect("copy into dest");

        assert_eq!(fs::read(&image).expect("read image"), NEW_CONTENT);
    }

    #[test]
    fn copy_into_dest_keeps_the_staged_file_when_the_copy_fails() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let mut staged = stage(&dir.path().join(IMAGE), NEW_CONTENT);
        // a character device cannot be resized, so the copy fails after the open
        staged.dest = PathBuf::from(UNRESIZABLE_FILE);

        assert!(staged.copy_into_dest().is_err());
        let file = staged.file.clone();
        drop(staged);
        assert_eq!(fs::read(&file).expect("read staged file"), NEW_CONTENT);
    }

    #[test]
    fn is_mount_root_detects_the_root_dir() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        touch(&image);

        assert!(is_mount_root(Path::new("/")));
        assert!(!is_mount_root(&image));
    }

    #[test]
    fn commit_replaces_the_target_of_a_symlinked_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        let target_bmap = dir.path().join("target.bmap");
        fs::write(&image, OLD_CONTENT).expect("write image");
        fs::write(&target_bmap, OLD_CONTENT).expect("write bmap");
        symlink(&target_bmap, &image_bmap).expect("create symlink");

        commit(
            stage(&image, NEW_CONTENT),
            Some(stage(&image_bmap, NEW_CONTENT)),
        )
        .expect("commit");

        assert!(image_bmap.is_symlink());
        assert_eq!(fs::read(&target_bmap).expect("read bmap"), NEW_CONTENT);
    }

    #[test]
    fn same_file_resolves_relative_and_missing_paths() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        touch(&image);
        let relative = dir.path().join(".").join(IMAGE);
        let missing = dir.path().join(OTHER_IMAGE);

        assert!(same_file(&image, &relative).expect("same file"));
        assert!(same_file(&missing, &missing).expect("same file"));
        assert!(!same_file(&image, &missing).expect("same file"));
    }
}
