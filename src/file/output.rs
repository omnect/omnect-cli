use crate::file::{
    bmap,
    compression::{BMAPTOOL_COMPRESSIONS, Compression},
    dir_of,
    signal::RegisteredTempDir,
    sparse,
};
use anyhow::{Context, Result};
use log::{info, warn};
use rustix::{
    fs::{Access, AtFlags, CWD, StatxAttributes, StatxFlags},
    io::Errno,
};
use std::{
    borrow::Borrow,
    collections::BTreeSet,
    env,
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind},
    iter,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use strum::IntoEnumIterator;

const TMP_DIR_PREFIX: &str = ".omnect-cli-";
// the limit of Linux path resolution
const MAX_SYMLINKS: usize = 40;

/// Returns `explicit`, else `image` if it has an extension of `target` that `bmaptool` reads, else
/// `image` without the extension of its `source` compression, with the extension of `target`
/// appended. Fails if `explicit` does not have an extension of `target` that `bmaptool` reads, or
/// has a compression extension while `target` is none.
pub fn output_image(
    image: &Path,
    source: Option<&Compression>,
    target: Option<&Compression>,
    explicit: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(path) = explicit {
        match target {
            Some(c) => anyhow::ensure!(
                c.matches_output_extension(path),
                "{} does not end in an extension of the -p compression that bmaptool reads; name \
                 it e.g. {}",
                path.display(),
                c.compressed_path(&without_extension_of(path, Compression::iter()))
                    .display()
            ),
            None => anyhow::ensure!(
                !Compression::has_extension(path),
                "{} has a compression extension, but the image is written uncompressed; pass -p",
                path.display()
            ),
        }
        return Ok(path.to_path_buf());
    }
    if target.is_some_and(|c| c.matches_output_extension(image)) {
        return Ok(image.to_path_buf());
    }

    let output = without_extension_of(image, source);
    Ok(match target {
        Some(c) => c.compressed_path(&output),
        None => output,
    })
}

/// `path` without its extension if it is an extension of one of `compressions`.
fn without_extension_of<C: Borrow<Compression>>(
    path: &Path,
    compressions: impl IntoIterator<Item = C>,
) -> PathBuf {
    let mut path = path.to_path_buf();
    if compressions
        .into_iter()
        .any(|c| c.borrow().matches_extension(&path))
    {
        path.set_extension("");
    }
    path
}

/// Fails if writing `output_image` and its bmap would overwrite a file the user did not name,
/// write a bmap over an image, write the output image over the input bmap or a bmap of an image
/// that stays, replace the bmap of an image that stays, leave a bmap that `bmaptool` would use
/// for the output image and that does not match it, write a file that cannot be replaced, or
/// read the input image as its own bmap. The checks that `--output-image` fixes come first, so a
/// hint does not name an option that a later check refuses.
pub fn check_conflicts(
    input_image: &Path,
    output_image: &Path,
    in_place: bool,
    explicit_output: bool,
    output_bmap: Option<&Path>,
    input_bmap: Option<&Path>,
) -> Result<()> {
    let outputs = || iter::once(output_image).chain(output_bmap);
    for path in outputs() {
        ensure_regular_file(path)?;
    }
    anyhow::ensure!(
        // a symlink counts also if its target does not exist yet
        explicit_output || in_place || output_image.symlink_metadata().is_err(),
        "{} exists; choose the output image with --output-image",
        output_image.display()
    );
    if let Some(target_bmap) = output_bmap {
        // e.g. an image named `x.bmap`, for which `bmaptool` finds the image itself as its bmap
        for image in [input_image, output_image] {
            anyhow::ensure!(
                !same_file(target_bmap, image)?,
                "{} would be replaced by the bmap of {}; choose another output image with \
                 --output-image",
                image.display(),
                output_image.display()
            );
        }
    }
    if !in_place {
        ensure_no_bmap_replaced(input_image, output_image, output_bmap, input_bmap)?;
    }
    let stale_bmap = match output_bmap {
        Some(target_bmap) => {
            ensure_bmap_next_to(output_image, target_bmap)?;
            ensure_used_only_for(target_bmap, output_image)?;
            None
        }
        None => find_stale_bmap(output_image, in_place)?,
    };
    for path in outputs() {
        ensure_replaceable(path)?;
    }
    if let Some(stale_bmap) = stale_bmap {
        anyhow::bail!(
            "{} would not match {}; pass -b to write a matching bmap",
            stale_bmap.display(),
            output_image.display()
        );
    }
    if let Some(bmap) = input_bmap {
        anyhow::ensure!(
            !same_file(bmap, input_image)?,
            "{} is both the input image and its input bmap; pass the right bmap with \
             --input-bmap, or map the whole image with --no-input-bmap",
            input_image.display()
        );
    }

    Ok(())
}

/// Fails if `bmaptool` would use `target_bmap` also for an image other than `output_image`.
fn ensure_used_only_for(target_bmap: &Path, output_image: &Path) -> Result<()> {
    for image in images_using(target_bmap)? {
        anyhow::ensure!(
            same_file(&image, output_image)?,
            "bmaptool would use {} also for {}, which stays and would not match it; choose \
             another output image with --output-image",
            target_bmap.display(),
            image.display()
        );
    }
    Ok(())
}

/// The bmap that `bmaptool` would use for `output_image` without `-b`, which would not match it.
/// Fails if `-b` would not help either: the bmap is the image itself, or `-b` would replace it
/// while it is also the bmap of an image that stays.
fn find_stale_bmap(output_image: &Path, in_place: bool) -> Result<Option<PathBuf>> {
    let Some(stale_bmap) = bmap::find_input_bmap(output_image, None)? else {
        return Ok(None);
    };
    anyhow::ensure!(
        !same_file(&stale_bmap, output_image)?,
        "bmaptool would use {} as its own bmap; choose another output image with \
         --output-image",
        output_image.display()
    );
    // else -b writes `<output image>.bmap`, which comes first in the lookup of `bmaptool`
    if in_place || same_file(&stale_bmap, &bmap::bmap_path(output_image))? {
        for image in images_using(&stale_bmap)? {
            anyhow::ensure!(
                same_file(&image, output_image)?,
                "{} is the bmap of {}, which stays, and would not match {}; choose another \
                 output image with --output-image",
                stale_bmap.display(),
                image.display(),
                output_image.display()
            );
        }
    }
    Ok(Some(stale_bmap))
}

/// Fails if the output image or its bmap would replace the input bmap, or if the output image
/// would replace or become a bmap that `bmaptool` uses for another image.
fn ensure_no_bmap_replaced(
    input_image: &Path,
    output_image: &Path,
    output_bmap: Option<&Path>,
    input_bmap: Option<&Path>,
) -> Result<()> {
    if let Some(bmap) = input_bmap {
        for output in iter::once(output_image).chain(output_bmap) {
            anyhow::ensure!(
                !same_file(bmap, output)?,
                "{} would replace the input bmap of {}; choose another output image with \
                 --output-image",
                output.display(),
                input_image.display()
            );
        }
    }
    if let Some(image) = images_using(output_image)?.first() {
        anyhow::bail!(
            "{} is or would become the bmap of {}, which stays; choose another output image \
             with --output-image",
            output_image.display(),
            image.display()
        );
    }
    Ok(())
}

/// The existing images next to `bmap` for which `bmaptool` would use `bmap`: `<stem>` and
/// `<stem>.<compression>`, if no other bmap comes first in the lookup of `bmaptool`. The stem is
/// taken from `bmap` and from its resolved path, because a symlinked bmap, e.g. a Yocto deploy
/// link, is also the bmap of the images named after its target.
fn images_using(bmap: &Path) -> Result<Vec<PathBuf>> {
    let resolved_bmap = resolve(bmap)?;
    let stems = [bmap.with_extension(""), resolved_bmap.with_extension("")];
    let candidates = stems.into_iter().flat_map(|stem| {
        iter::once(stem.clone()).chain(
            BMAPTOOL_COMPRESSIONS
                .iter()
                .map(move |extension| stem.with_added_extension(extension)),
        )
    });
    let mut images = vec![];
    // both stems name the same images if `bmap` is no symlink
    let mut seen = BTreeSet::new();
    for image in candidates.filter(|image| image.is_file()) {
        let resolved = resolve(&image)?;
        // e.g. the link `x.wic.bmap` to `x.wic.bmap.orig`
        if resolved == resolved_bmap || !seen.insert(resolved) {
            continue;
        }
        for lookup in bmap::bmaptool_lookup(&image) {
            if same_file(&lookup, bmap)? {
                images.push(image);
                break;
            }
            if lookup.is_file() {
                break;
            }
        }
    }
    Ok(images)
}

/// Fails if `output_image` is a mount point, e.g. an image file mounted into a container, and
/// `bmap` is not: the bmap would then be written next to the mount point, not next to the image.
fn ensure_bmap_next_to(output_image: &Path, bmap: &Path) -> Result<()> {
    anyhow::ensure!(
        !is_mount_root(&resolve(output_image)?) || is_mount_root(&resolve(bmap)?),
        "{} is a mount point, so its bmap {} would not be written next to the image; mount the \
         directory of the image instead",
        output_image.display(),
        bmap.display()
    );
    Ok(())
}

/// `<output image>.bmap`, but for an image changed in place the bmap that `bmaptool` finds next
/// to it, e.g. `x.wic.bmap` for `x.wic.xz`, so no bmap of the old image stays.
pub fn output_bmap_path(output_image: &Path, in_place: bool, found_bmap: Option<&Path>) -> PathBuf {
    match found_bmap {
        Some(bmap) if in_place => bmap.to_path_buf(),
        _ => bmap::bmap_path(output_image),
    }
}

/// Fails if `Staged::write` and `commit` could not replace the regular file `path` for lack of
/// permissions.
fn ensure_replaceable(path: &Path) -> Result<()> {
    ensure_writable(path)?;
    ensure_staging_dir_writable(path)
}

/// Fails if `path` exists and is no regular file, e.g. a block device, which the rename of the
/// staged file would replace instead of writing to it.
fn ensure_regular_file(path: &Path) -> Result<()> {
    match fs::metadata(path) {
        Ok(metadata) => anyhow::ensure!(
            metadata.is_file(),
            "{} is not a regular file; choose another output image with --output-image",
            path.display()
        ),
        Err(e) => anyhow::ensure!(
            e.kind() == ErrorKind::NotFound,
            "cannot read {}: {e}",
            path.display()
        ),
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

/// Fails before the command runs if `Staged::write` could not create its temp dir for `dest`.
fn ensure_staging_dir_writable(dest: &Path) -> Result<()> {
    let (dir, _) = staging_dir(&resolve(dest)?)?;
    rustix::fs::access(&dir, Access::WRITE_OK).context(format!(
        "cannot write to {}, where the new {} is staged; make it writable or choose another \
         output image with --output-image",
        dir.display(),
        dest.display()
    ))
}

/// The directory for the staged file of the resolved `dest`, and whether the staged file is
/// copied into `dest`. A rename cannot replace a mount point, and needs write access to the
/// directory of `dest`, so an existing `dest` without either is staged in TMPDIR.
fn staging_dir(dest: &Path) -> Result<(PathBuf, bool)> {
    let dir = dest
        .parent()
        .context(format!("{} has no directory", dest.display()))?;
    let dir_writable = rustix::fs::access(dir, Access::WRITE_OK).is_ok();
    if is_mount_root(dest) || (dest.exists() && !dir_writable) {
        return Ok((env::temp_dir(), true));
    }
    Ok((dir.to_path_buf(), false))
}

/// A new file in the directory of its destination, which `commit` renames to the destination.
/// Until then the destination stays unchanged. A rename cannot replace a mount point, e.g. an
/// image file mounted into a container, or a file in a directory the user cannot write, so for
/// such a destination the new file is staged in TMPDIR and copied into it.
pub struct Staged {
    tmp_dir: RegisteredTempDir,
    file: PathBuf,
    dest: PathBuf,
    in_tmpdir: bool,
}

impl Staged {
    /// Writes and syncs the new file for `dest`. A symlink is followed, and an existing `dest`
    /// keeps its permissions.
    pub fn write(dest: &Path, write: impl FnOnce(&mut File) -> Result<()>) -> Result<Self> {
        let dest = resolve(dest)?;
        let (tmp_parent, in_tmpdir) = staging_dir(&dest)?;
        let tmp_dir = RegisteredTempDir::create_in(&tmp_parent, TMP_DIR_PREFIX).context(
            format!("cannot create a temp dir in {}", tmp_parent.display()),
        )?;
        let file = tmp_dir.path().join(
            dest.file_name()
                .context(format!("{} has no file name", dest.display()))?,
        );
        let staged = Self {
            tmp_dir,
            file,
            dest,
            in_tmpdir,
        };

        let mut handle = File::create(&staged.file)
            .context(format!("cannot create {}", staged.file.display()))?;
        write(&mut handle)?;
        if let Ok(metadata) = fs::metadata(&staged.dest) {
            handle
                .set_permissions(metadata.permissions())
                .context(format!(
                    "cannot set permissions of {}",
                    staged.file.display()
                ))?;
        }
        // a file in TMPDIR is only copied into `dest`, and the copy is synced
        if !in_tmpdir {
            // the handle that wrote the data also gets the write errors
            handle
                .sync_all()
                .context(format!("cannot sync {}", staged.file.display()))?;
        }

        Ok(staged)
    }

    fn rename(&mut self) -> Result<()> {
        if self.in_tmpdir {
            return self.copy_into_dest();
        }
        match fs::rename(&self.file, &self.dest) {
            // a sticky dir, e.g. /tmp, refuses to replace a file of another user
            Err(e)
                if matches!(
                    e.kind(),
                    ErrorKind::ResourceBusy
                        | ErrorKind::CrossesDevices
                        | ErrorKind::PermissionDenied
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

    /// Keeps the staged file if the copy fails or a signal stops it, because `dest` is truncated
    /// already.
    fn copy_into_dest(&mut self) -> Result<()> {
        self.tmp_dir.unregister();
        info!(
            "copying {} into {}; it stays if the copy fails",
            self.file.display(),
            self.dest.display()
        );
        let open = |create_new| {
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .create_new(create_new)
                // `dest` is resolved already; a symlink here was put in its place since then
                .custom_flags(libc::O_NOFOLLOW)
                .open(&self.dest)
        };
        // O_CREAT only for a new `dest`: `fs.protected_regular` refuses it for an existing file
        // of another user in a sticky dir
        let dest = match open(false) {
            Err(e) if e.kind() == ErrorKind::NotFound => open(true),
            result => result,
        }
        .context(format!("cannot open {}", self.dest.display()))?;

        let result = sparse::copy_file(&self.file, &dest).and_then(|()| {
            dest.sync_all()
                .context(format!("cannot sync {}", self.dest.display()))
        });
        match result {
            Ok(()) => self.tmp_dir.register(),
            Err(_) => self.tmp_dir.keep(),
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

/// Renames the staged `image` and then its `bmap`, then syncs their directories, so the renames
/// are not lost on a power loss. If the image fails, the old image keeps its bmap, unless the
/// image was truncated already: then the old bmap is removed and the new one is kept next to the
/// kept image. If the bmap fails, the old bmap is removed, because it does not match the new
/// image. An old bmap that cannot be removed is named in the error. A stop signal after the image
/// step removes the staged bmap and keeps the old one.
pub fn commit(mut image: Staged, mut bmap: Option<Staged>) -> Result<()> {
    if let Err(e) = image.rename() {
        return match &bmap {
            Some(bmap) if image.tmp_dir.is_kept() => Err(keep_next_to(bmap, &image, e)),
            _ => Err(e),
        };
    }
    if let Some(bmap) = &mut bmap
        && let Err(e) = bmap.rename()
    {
        return Err(remove_old_bmap(bmap, e));
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

/// Copies the staged `bmap` next to the kept staged `image` and removes the old bmap, which
/// matches no image anymore.
fn keep_next_to(bmap: &Staged, image: &Staged, error: anyhow::Error) -> anyhow::Error {
    let error = remove_old_bmap(bmap, error);
    let Some(name) = bmap.dest.file_name() else {
        return error;
    };
    let kept = image.tmp_dir.path().join(name);
    match fs::copy(&bmap.file, &kept) {
        Ok(_) => error.context(format!("the new bmap is kept in {}", kept.display())),
        Err(e) => {
            warn!("cannot keep the new bmap in {}: {e}", kept.display());
            error
        }
    }
}

/// Removes the old bmap at the destination of `bmap`, and adds to `error` if it stays. A kept
/// `bmap` was truncated already by the copy into its destination.
fn remove_old_bmap(bmap: &Staged, error: anyhow::Error) -> anyhow::Error {
    let Err(e) = remove_old(&bmap.dest) else {
        return error;
    };
    let state = if bmap.tmp_dir.is_kept() {
        "is incomplete"
    } else {
        "does not match the new image"
    };
    error.context(format!(
        "the old {} stays and {state}: {e}",
        bmap.dest.display()
    ))
}

fn remove_old(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

pub(crate) fn same_file(a: &Path, b: &Path) -> Result<bool> {
    Ok(resolve(a)? == resolve(b)?)
}

/// The absolute path of `path` with symlinks resolved. A file that does not exist yet, also as
/// the target of a symlink, is resolved through its directory.
fn resolve(path: &Path) -> Result<PathBuf> {
    let mut path = path.to_path_buf();
    for _ in 0..MAX_SYMLINKS {
        if !path.is_symlink() || path.exists() {
            return resolve_existing_dir(&path);
        }
        let target = fs::read_link(&path).context(format!("cannot read {}", path.display()))?;
        // a relative target is relative to the directory of the link
        path = path.parent().unwrap_or(Path::new("")).join(target);
    }
    anyhow::bail!("too many symlinks in {}", path.display())
}

fn resolve_existing_dir(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return fs::canonicalize(path).context(format!("cannot resolve {}", path.display()));
    }
    let dir = dir_of(path);
    let name = path
        .file_name()
        .context(format!("{} has no file name", path.display()))?;

    Ok(fs::canonicalize(dir)
        .context(format!("cannot resolve {}", dir.display()))?
        .join(name))
}

#[cfg(test)]
mod tests {
    use crate::file::{output::*, signal};
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
    const DIR_MODE: u32 = 0o755;
    const READ_ONLY_DIR_MODE: u32 = 0o555;
    const UNRESIZABLE_FILE: &str = "/dev/full";
    const DEVICE_FILE: &str = "/dev/null";
    const MOUNTINFO: &str = "/proc/self/mountinfo";
    const MOUNTINFO_MOUNT_POINT: usize = 4;

    fn bmap_of(input_image: &Path, output_image: &Path) -> PathBuf {
        let found_bmap = bmap::find_input_bmap(input_image, None).expect("find bmap");
        output_bmap_path(
            output_image,
            same_file(input_image, output_image).expect("same file"),
            found_bmap.as_deref(),
        )
    }

    fn conflicts(
        input_image: &Path,
        output_image: &Path,
        explicit_output: bool,
        output_bmap: Option<&Path>,
        input_bmap: Option<&Path>,
    ) -> Result<()> {
        check_conflicts(
            input_image,
            output_image,
            same_file(input_image, output_image).expect("same file"),
            explicit_output,
            output_bmap,
            input_bmap,
        )
    }

    /// Makes a dir read-only until drop, also if the test panics, so the temp dir can be
    /// removed.
    struct ReadOnlyDir<'a>(&'a Path);

    impl<'a> ReadOnlyDir<'a> {
        fn new(dir: &'a Path) -> Self {
            fs::set_permissions(dir, fs::Permissions::from_mode(READ_ONLY_DIR_MODE))
                .expect("set permissions");
            Self(dir)
        }
    }

    impl Drop for ReadOnlyDir<'_> {
        fn drop(&mut self) {
            if let Err(e) = fs::set_permissions(self.0, fs::Permissions::from_mode(DIR_MODE)) {
                eprintln!("cannot make {} writable: {e}", self.0.display());
            }
        }
    }

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
    fn output_image_uses_the_extension_bmaptool_reads() {
        for (image, compression, output) in [
            ("image.wic.gz", Compression::gzip, "image.wic.gz"),
            ("image.wic.gzip", Compression::gzip, "image.wic.gzip"),
            ("image.wic.bz2", Compression::bzip2, "image.wic.bz2"),
            ("image.wic.bzip2", Compression::bzip2, "image.wic.bz2"),
        ] {
            assert_eq!(
                output_image(
                    Path::new(image),
                    Some(&compression),
                    Some(&compression),
                    None
                )
                .expect("output image"),
                Path::new(output)
            );
        }
        let error = output_image(
            Path::new(IMAGE),
            None,
            Some(&Compression::bzip2),
            Some(Path::new("device.wic.bzip2")),
        )
        .expect_err("long name with -o");
        assert!(error.to_string().contains("name it e.g. device.wic.bz2"));
        let gz = Path::new("device.wic.gz");
        assert_eq!(
            output_image(Path::new(IMAGE), None, Some(&Compression::gzip), Some(gz))
                .expect("short gzip name with -o"),
            gz
        );
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

        conflicts(
            &image,
            &image,
            false,
            Some(&bmap_of(&image, &image)),
            Some(&image_bmap),
        )
        .expect("no conflict");
    }

    #[test]
    fn check_conflicts_rejects_an_image_that_is_its_own_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join("image.bmap");
        touch(&image);

        // bmaptool finds the image itself also as its input bmap
        let error = conflicts(
            &image,
            &image,
            false,
            Some(&bmap_of(&image, &image)),
            Some(&image),
        )
        .expect_err("with -b");
        assert!(
            error
                .to_string()
                .contains("would be replaced by the bmap of")
        );
        let error = conflicts(&image, &image, false, None, None).expect_err("without -b");
        assert!(
            error
                .to_string()
                .contains("as its own bmap; choose another output image")
        );
    }

    #[test]
    fn check_conflicts_rejects_an_output_bmap_that_is_the_input_image() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let input = dir.path().join("image.bmap");
        let output = dir.path().join("image");
        touch(&input);

        let error = conflicts(
            &input,
            &output,
            true,
            Some(&bmap_of(&input, &output)),
            Some(&input),
        )
        .expect_err("output bmap is the input image");
        assert!(
            error
                .to_string()
                .contains(&format!("{} would be replaced", input.display()))
        );
    }

    #[test]
    fn check_conflicts_rejects_an_input_bmap_that_is_the_input_image() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let input = dir.path().join("image.bmap");
        let output = dir.path().join(OTHER_IMAGE);
        touch(&input);

        let error = conflicts(
            &input,
            &output,
            true,
            Some(&bmap_of(&input, &output)),
            Some(&input),
        )
        .expect_err("input bmap is the input image");
        assert!(
            error
                .to_string()
                .contains("is both the input image and its input bmap")
        );
    }

    #[test]
    fn check_conflicts_rejects_an_output_image_that_is_the_input_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        touch(&image);
        touch(&image_bmap);
        let output_bmap = bmap_of(&image, &image_bmap);

        // -b, -b --no-input-bmap and no -b
        for (output_bmap, input_bmap, message) in [
            (
                Some(output_bmap.as_path()),
                Some(image_bmap.as_path()),
                format!("would replace the input bmap of {}", image.display()),
            ),
            (
                Some(output_bmap.as_path()),
                None,
                format!("is or would become the bmap of {}", image.display()),
            ),
            (
                None,
                None,
                format!("is or would become the bmap of {}", image.display()),
            ),
        ] {
            let error = conflicts(&image, &image_bmap, true, output_bmap, input_bmap)
                .expect_err("output image is the input bmap");
            assert!(error.to_string().contains(&message), "{error}");
        }
    }

    #[test]
    fn check_conflicts_names_a_bmap_conflict_before_a_read_only_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        touch(&image_xz);
        touch(&image_bmap);
        fs::set_permissions(&image_bmap, fs::Permissions::from_mode(READ_ONLY_MODE))
            .expect("set permissions");

        let error = conflicts(
            &image_xz,
            &image,
            false,
            Some(&bmap_of(&image_xz, &image)),
            Some(&image_bmap),
        )
        .expect_err("input bmap would be replaced");
        assert!(
            error.to_string().contains("would replace the input bmap"),
            "{error}"
        );
    }

    #[test]
    fn check_conflicts_asks_for_b_when_the_new_bmap_comes_first() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let image_xz = dir.path().join(IMAGE_XZ);
        let image_bmap = bmap::bmap_path(&image);
        touch(&image);
        touch(&image_bmap);

        // -i image.wic -p xz: -b writes image.wic.xz.bmap, which bmaptool finds first
        let error = conflicts(&image, &image_xz, false, None, None).expect_err("without -b");
        assert!(error.to_string().contains("pass -b"), "{error}");
        conflicts(
            &image,
            &image_xz,
            false,
            Some(&bmap_of(&image, &image_xz)),
            Some(&image_bmap),
        )
        .expect("with -b");
    }

    #[test]
    fn check_conflicts_points_to_output_image_for_the_stale_bmap_of_an_image_that_stays() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image = dir.path().join(IMAGE);
        touch(&image_xz);
        touch(&bmap::bmap_path(&image));

        let error = conflicts(&image_xz, &image, false, None, None).expect_err("stale bmap");
        assert!(error.to_string().contains(&format!(
            "is the bmap of {}, which stays",
            image_xz.display()
        )));
    }

    #[test]
    fn output_bmap_path_replaces_the_bmap_bmaptool_finds_in_place() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image_bmap = bmap::bmap_path(&dir.path().join(IMAGE));
        let other = dir.path().join(OTHER_IMAGE);
        touch(&image_xz);

        let output = |output| bmap_of(&image_xz, output);
        assert_eq!(output(&image_xz), bmap::bmap_path(&image_xz));
        touch(&image_bmap);
        assert_eq!(output(&image_xz), image_bmap);
        assert_eq!(output(&other), bmap::bmap_path(&other));
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

        assert!(
            conflicts(
                &image,
                &image,
                false,
                Some(&bmap_of(&image, &image)),
                Some(&image_bmap)
            )
            .is_err()
        );
        fs::remove_file(&image_bmap).expect("remove bmap");
        conflicts(&image, &image, false, None, None).expect("writable image");
        fs::set_permissions(&image, read_only).expect("set permissions");
        assert!(conflicts(&image, &image, false, None, None).is_err());
    }

    #[test]
    fn check_conflicts_allows_only_in_place_in_a_read_only_dir() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        touch(&image);
        let read_only = ReadOnlyDir::new(dir.path());
        // root can write a read-only directory
        let writable = rustix::fs::access(dir.path(), Access::WRITE_OK).is_ok();

        let in_place = conflicts(&image, &image, false, None, None);
        let new_file = conflicts(&image, &dir.path().join(OTHER_IMAGE), true, None, None);
        drop(read_only);
        in_place.expect("in place");
        assert_eq!(new_file.is_ok(), writable);
    }

    #[test]
    fn staged_commit_copies_into_a_file_in_a_read_only_dir() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        fs::write(&image, OLD_CONTENT).expect("write image");
        let read_only = ReadOnlyDir::new(dir.path());
        let writable = rustix::fs::access(dir.path(), Access::WRITE_OK).is_ok();

        let staged = stage(&image, NEW_CONTENT);
        let in_tmpdir = staged.in_tmpdir;
        let result = commit(staged, None);
        drop(read_only);
        result.expect("commit");
        assert_eq!(in_tmpdir, !writable);
        assert_eq!(fs::read(&image).expect("read image"), NEW_CONTENT);
    }

    #[test]
    fn check_conflicts_rejects_an_output_that_is_no_regular_file() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let device = dir.path().join(OTHER_IMAGE);
        let output_dir = dir.path().join("output.wic");
        touch(&image);
        symlink(DEVICE_FILE, &device).expect("create symlink");
        fs::create_dir(&output_dir).expect("create dir");

        for output in [&device, &output_dir] {
            let error = conflicts(&image, output, true, None, None).expect_err("no file");
            assert!(error.to_string().contains("is not a regular file"));
        }
    }

    #[test]
    fn check_conflicts_rejects_existing_derived_image() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image = dir.path().join(IMAGE);
        touch(&image_xz);
        touch(&image);

        assert!(conflicts(&image_xz, &image, false, None, None).is_err());
        conflicts(&image_xz, &image, true, None, None).expect("explicit output");
    }

    #[test]
    fn check_conflicts_rejects_replacing_the_input_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        touch(&image_xz);
        touch(&image_bmap);

        assert!(
            conflicts(
                &image_xz,
                &image,
                false,
                Some(&bmap_of(&image_xz, &image)),
                Some(&image_bmap)
            )
            .is_err()
        );
        let other = dir.path().join(OTHER_IMAGE);
        conflicts(
            &image_xz,
            &other,
            true,
            Some(&bmap_of(&image_xz, &other)),
            Some(&image_bmap),
        )
        .expect("other output");
    }

    #[test]
    fn check_conflicts_rejects_leaving_a_stale_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        touch(&image);
        touch(&bmap::bmap_path(&image));

        assert!(conflicts(&image, &image, false, None, None).is_err());
    }

    #[test]
    fn check_conflicts_rejects_a_stale_bmap_bmaptool_finds() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        touch(&image);
        touch(&dir.path().join("image.bmap"));

        assert!(conflicts(&image, &image, false, None, None).is_err());
    }

    #[test]
    fn check_conflicts_rejects_a_stale_bmap_of_the_uncompressed_image() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image_bmap = bmap::bmap_path(&dir.path().join(IMAGE));
        touch(&image_xz);
        touch(&image_bmap);

        // bmaptool uses image.wic.bmap for image.wic.xz
        assert!(conflicts(&image_xz, &image_xz, false, None, None).is_err());
        conflicts(
            &image_xz,
            &image_xz,
            false,
            Some(&bmap_of(&image_xz, &image_xz)),
            Some(&image_bmap),
        )
        .expect("with -b");
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
        assert!(
            conflicts(
                &image_xz,
                &image,
                false,
                Some(&bmap_of(&image_xz, &image)),
                Some(&other_bmap)
            )
            .is_err()
        );
        assert!(
            conflicts(
                &image_xz,
                &image,
                false,
                Some(&bmap_of(&image_xz, &image)),
                None
            )
            .is_err()
        );
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
    fn staged_registers_its_temp_dir_until_drop() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let staged = stage(&dir.path().join(IMAGE), NEW_CONTENT);
        let tmp_dir = staged.tmp_dir.path().to_path_buf();

        assert!(signal::is_registered(&tmp_dir));
        drop(staged);
        assert!(!signal::is_registered(&tmp_dir));
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
    fn commit_names_an_old_bmap_that_stays_when_the_bmap_rename_fails() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        fs::write(&image, OLD_CONTENT).expect("write image");

        let staged_bmap = stage(&image_bmap, NEW_CONTENT);
        fs::remove_file(&staged_bmap.file).expect("remove staged bmap");
        // a dir cannot be removed as a file
        fs::create_dir(&image_bmap).expect("create dir in place of the bmap");

        let error = commit(stage(&image, NEW_CONTENT), Some(staged_bmap)).expect_err("commit");
        assert!(format!("{error:#}").contains("stays and does not match"));
    }

    #[test]
    fn commit_names_an_old_bmap_that_stays_next_to_an_image_kept_after_a_failed_copy() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        let staged_bmap = stage(&image_bmap, NEW_CONTENT);
        // a dir cannot be removed as a file
        fs::create_dir(&image_bmap).expect("create dir in place of the bmap");
        let mut staged_image = stage(&image, NEW_CONTENT);
        staged_image.dest = PathBuf::from(UNRESIZABLE_FILE);
        staged_image.in_tmpdir = true;

        let error = commit(staged_image, Some(staged_bmap)).expect_err("commit fails");
        assert!(format!("{error:#}").contains("stays and does not match"));
    }

    #[test]
    fn remove_old_bmap_names_a_truncated_bmap_incomplete() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_bmap = bmap::bmap_path(&dir.path().join(IMAGE));
        let mut staged_bmap = stage(&image_bmap, NEW_CONTENT);
        fs::create_dir(&image_bmap).expect("create dir in place of the bmap");
        staged_bmap.tmp_dir.keep();

        let error = remove_old_bmap(&staged_bmap, anyhow::anyhow!("copy failed"));
        assert!(format!("{error:#}").contains("stays and is incomplete"));
    }

    #[test]
    fn commit_keeps_the_old_bmap_when_the_image_rename_fails() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        fs::write(&image, OLD_CONTENT).expect("write image");
        fs::write(&image_bmap, OLD_CONTENT).expect("write bmap");

        let staged_image = stage(&image, NEW_CONTENT);
        fs::remove_file(&staged_image.file).expect("remove staged image");

        assert!(commit(staged_image, Some(stage(&image_bmap, NEW_CONTENT))).is_err());
        assert_eq!(fs::read(&image).expect("read image"), OLD_CONTENT);
        assert_eq!(fs::read(&image_bmap).expect("read bmap"), OLD_CONTENT);
    }

    #[test]
    fn copy_into_dest_replaces_the_content() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        fs::write(&image, b"old image with more bytes").expect("write image");

        let mut staged = stage(&image, NEW_CONTENT);
        staged.copy_into_dest().expect("copy into dest");

        assert_eq!(fs::read(&image).expect("read image"), NEW_CONTENT);
        assert!(signal::is_registered(staged.tmp_dir.path()));
    }

    #[test]
    fn copy_into_dest_does_not_follow_a_symlink_put_in_place_of_dest() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let other = dir.path().join(OTHER_IMAGE);
        fs::write(&image, OLD_CONTENT).expect("write image");
        fs::write(&other, OLD_CONTENT).expect("write other image");
        let mut staged = stage(&image, NEW_CONTENT);
        fs::remove_file(&image).expect("remove image");
        symlink(&other, &image).expect("create symlink");

        assert!(staged.copy_into_dest().is_err());
        assert_eq!(fs::read(&other).expect("read other image"), OLD_CONTENT);
    }

    #[test]
    fn copy_into_dest_creates_a_missing_dest() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let mut staged = stage(&image, NEW_CONTENT);

        staged.copy_into_dest().expect("copy into dest");
        assert_eq!(fs::read(&image).expect("read image"), NEW_CONTENT);
    }

    #[test]
    fn copy_into_dest_keeps_the_staged_file_when_the_copy_fails() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let mut staged = stage(&dir.path().join(IMAGE), NEW_CONTENT);
        // a character device cannot be resized, so the copy fails after the open
        staged.dest = PathBuf::from(UNRESIZABLE_FILE);

        assert!(staged.copy_into_dest().is_err());
        assert!(!signal::is_registered(staged.tmp_dir.path()));
        let file = staged.file.clone();
        drop(staged);
        assert_eq!(fs::read(&file).expect("read staged file"), NEW_CONTENT);
    }

    #[test]
    fn commit_keeps_the_new_bmap_next_to_an_image_kept_after_a_failed_copy() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        fs::write(&image_bmap, OLD_CONTENT).expect("write bmap");
        let mut staged_image = stage(&image, NEW_CONTENT);
        staged_image.dest = PathBuf::from(UNRESIZABLE_FILE);
        staged_image.in_tmpdir = true;
        let kept_bmap = staged_image
            .file
            .with_file_name(image_bmap.file_name().expect("bmap name"));

        let e =
            commit(staged_image, Some(stage(&image_bmap, NEW_CONTENT))).expect_err("commit fails");

        assert!(format!("{e:#}").contains(&kept_bmap.display().to_string()));
        assert_eq!(fs::read(&kept_bmap).expect("read kept bmap"), NEW_CONTENT);
        assert!(!image_bmap.exists());
    }

    #[test]
    fn is_mount_root_detects_the_root_dir() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        touch(&image);

        assert!(!is_mount_root(&image));

        // kernels before 5.8 do not report mount roots, and `/` of a chroot is no mount root
        let root = Path::new("/");
        let reported = rustix::fs::statx(CWD, root, AtFlags::empty(), StatxFlags::empty())
            .is_ok_and(|statx| {
                statx
                    .stx_attributes_mask
                    .contains(StatxAttributes::MOUNT_ROOT)
            });
        let mounted = fs::read_to_string(MOUNTINFO).is_ok_and(|mountinfo| {
            mountinfo
                .lines()
                .any(|line| line.split(' ').nth(MOUNTINFO_MOUNT_POINT) == Some("/"))
        });
        if reported && mounted {
            assert!(is_mount_root(root));
        }
    }

    #[test]
    fn ensure_bmap_next_to_rejects_a_bmap_next_to_a_mount_point() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let root = Path::new("/");
        touch(&image);

        ensure_bmap_next_to(&image, &bmap::bmap_path(&image)).expect("no mount point");
        // the kernel cannot always tell, see is_mount_root_detects_the_root_dir
        if is_mount_root(root) {
            assert!(ensure_bmap_next_to(root, &bmap::bmap_path(&image)).is_err());
            ensure_bmap_next_to(root, root).expect("bmap is a mount point too");
        }
    }

    #[test]
    fn check_conflicts_rejects_replacing_the_bmap_of_another_image() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        touch(&image_xz);
        touch(&image);
        touch(&image_bmap);

        // bmaptool uses image.wic.bmap for both images
        for input in [&image_xz, &image] {
            let error =
                conflicts(input, input, false, Some(&image_bmap), None).expect_err("shared bmap");
            assert!(error.to_string().contains("which stays"));
        }
        touch(&bmap::bmap_path(&image_xz));
        conflicts(&image, &image, false, Some(&image_bmap), None).expect("own bmaps");
        fs::remove_file(&image_bmap).expect("remove bmap");
        fs::remove_file(bmap::bmap_path(&image_xz)).expect("remove bmap");
        // the new bmap would be the first bmap that bmaptool finds for image.wic.xz
        assert!(conflicts(&image, &image, false, Some(&image_bmap), None).is_err());
    }

    #[test]
    fn images_using_finds_the_images_bmaptool_maps_to_a_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        for path in [
            &image_xz,
            &image,
            &image_bmap,
            &dir.path().join("image.wicked"),
            &dir.path().join("image.wic.xz.sha256"),
            &dir.path().join("image.wic.bmap.bak"),
        ] {
            touch(path);
        }

        let mut images = images_using(&image_bmap).expect("images");
        images.sort();
        assert_eq!(images, vec![image, image_xz]);
    }

    #[test]
    fn images_using_finds_the_images_named_after_the_target_of_a_symlinked_bmap() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        let link_bmap = dir.path().join("latest.wic.bmap");
        touch(&image);
        touch(&image_bmap);
        symlink(&image_bmap, &link_bmap).expect("create symlink");

        let images = images_using(&link_bmap).expect("images");
        assert_eq!(images.len(), 1);
        assert!(same_file(&images[0], &image).expect("same file"));
    }

    #[test]
    fn images_using_skips_a_bmap_link_named_after_its_target() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image = dir.path().join(IMAGE);
        let image_bmap = bmap::bmap_path(&image);
        let target = image_bmap.with_added_extension("orig");
        touch(&image);
        touch(&target);
        symlink(&target, &image_bmap).expect("create symlink");

        assert_eq!(images_using(&image_bmap).expect("images"), vec![image]);
    }

    #[test]
    fn images_using_lists_each_image_once_through_a_symlinked_dir() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let real = dir.path().join("real");
        let link = dir.path().join("link");
        fs::create_dir(&real).expect("create dir");
        symlink(&real, &link).expect("create symlink");
        for name in [IMAGE, IMAGE_XZ, "image.wic.bmap"] {
            touch(&real.join(name));
        }

        let mut images = images_using(&link.join("image.wic.bmap")).expect("images");
        images.sort();
        assert_eq!(images, vec![link.join(IMAGE), link.join(IMAGE_XZ)]);
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
    fn staged_commit_creates_the_target_of_a_dangling_symlink() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let target = dir.path().join("target").join(IMAGE);
        let link = dir.path().join(IMAGE);
        fs::create_dir(target.parent().expect("target dir")).expect("create dir");
        symlink(Path::new("target").join(IMAGE), &link).expect("create symlink");

        commit(stage(&link, NEW_CONTENT), None).expect("commit");

        assert!(link.is_symlink());
        assert_eq!(fs::read(&target).expect("read target"), NEW_CONTENT);
    }

    #[test]
    fn check_conflicts_rejects_a_dangling_derived_output() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let image_xz = dir.path().join(IMAGE_XZ);
        let image = dir.path().join(IMAGE);
        touch(&image_xz);
        symlink(dir.path().join("missing.wic"), &image).expect("create symlink");

        assert!(conflicts(&image_xz, &image, false, None, None).is_err());
    }

    #[test]
    fn resolve_stops_at_a_symlink_loop() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let link = dir.path().join(IMAGE);
        symlink(&link, &link).expect("create symlink");

        assert!(resolve(&link).is_err());
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
