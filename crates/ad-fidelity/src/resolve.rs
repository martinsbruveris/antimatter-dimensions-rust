//! Resolving a short save/fixture identifier to a concrete file path.
//!
//! The same convention is used across the harness (and mirrored in the JS oracle
//! script) so a save or fixture can be named the same way everywhere. Given an
//! identifier `id`, a directory `dir`, and an extension `ext`, resolution tries,
//! in order:
//!
//! 1. `id` as a path (relative or absolute), if it names an existing file;
//! 2. `dir/id`, if it names an existing file;
//! 3. the glob `dir/0*<id>-*.<ext>` — a captured file whose zero-padded leading
//!    index, with leading zeros stripped, equals `id` (so `id = "1"` matches
//!    `00001-…json` but not `00010-…json`).
//!
//! The glob must be unambiguous: zero matches is a [`ResolveError::NotFound`] and
//! two or more is a [`ResolveError::Ambiguous`] that lists the candidates, so the
//! caller can ask the user to disambiguate rather than silently guessing.

use std::fmt;
use std::path::{Path, PathBuf};

/// Why an identifier failed to resolve to exactly one file.
#[derive(Debug)]
pub enum ResolveError {
    /// No path, `dir/id`, or glob match named an existing file.
    NotFound {
        id: String,
        dir: PathBuf,
        ext: String,
    },
    /// The glob matched more than one file; the caller must disambiguate.
    Ambiguous { id: String, matches: Vec<PathBuf> },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::NotFound { id, dir, ext } => {
                let d = dir.display();
                write!(
                    f,
                    "no file matching id {id:?} \
                     (looked for path {id:?}, {d}/{id}, and {d}/0*{id}-*.{ext})"
                )
            }
            ResolveError::Ambiguous { id, matches } => {
                let names: Vec<String> = matches
                    .iter()
                    .map(|p| {
                        p.file_name()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_else(|| p.display().to_string())
                    })
                    .collect();
                write!(
                    f,
                    "id {id:?} is ambiguous — {} candidates: {}. \
                     Use a longer id or a full path.",
                    names.len(),
                    names.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// Resolve `id` to a single file under `dir` (extension `ext`, without the dot).
/// See the module docs for the precedence order.
pub fn resolve(id: &str, dir: &Path, ext: &str) -> Result<PathBuf, ResolveError> {
    // 1. `id` as a path.
    let as_path = Path::new(id);
    if as_path.is_file() {
        return Ok(as_path.to_path_buf());
    }

    // 2. `dir/id`.
    let joined = dir.join(id);
    if joined.is_file() {
        return Ok(joined);
    }

    // 3. glob `0*<id>-*.<ext>`. A missing `dir` simply yields no matches (and so
    // the friendlier NotFound below, listing what we looked for).
    let mut matches: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .filter(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|name| matches_id(name, id, ext))
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    matches.sort();

    match matches.len() {
        0 => Err(ResolveError::NotFound {
            id: id.to_string(),
            dir: dir.to_path_buf(),
            ext: ext.to_string(),
        }),
        1 => Ok(matches.pop().unwrap()),
        _ => Err(ResolveError::Ambiguous {
            id: id.to_string(),
            matches,
        }),
    }
}

/// Does `name` match `0*<id>-*.<ext>`? The leading number segment (everything
/// before the first `-`) with leading zeros stripped must equal `id`, and the
/// name must end in `.<ext>`. An all-zero segment reduces to `"0"`.
fn matches_id(name: &str, id: &str, ext: &str) -> bool {
    if !name.ends_with(&format!(".{ext}")) {
        return false;
    }
    let num = name.split('-').next().unwrap_or("");
    let significant = num.trim_start_matches('0');
    let significant = if significant.is_empty() {
        "0"
    } else {
        significant
    };
    significant == id
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A scratch directory unique to this test process + a caller-supplied tag.
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("ad-fidelity-resolve-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(dir: &Path, name: &str) {
        fs::write(dir.join(name), "x").unwrap();
    }

    #[test]
    fn glob_matches_zero_padded_index() {
        let dir = scratch("glob");
        touch(&dir, "00001-0000-11-44-timed.json");
        touch(&dir, "00010-0000-10-21-timed.json");
        touch(&dir, "00033-0050-08-26-manual.json");

        assert_eq!(
            resolve("1", &dir, "json").unwrap().file_name().unwrap(),
            "00001-0000-11-44-timed.json"
        );
        assert_eq!(
            resolve("10", &dir, "json").unwrap().file_name().unwrap(),
            "00010-0000-10-21-timed.json"
        );
        assert_eq!(
            resolve("33", &dir, "json").unwrap().file_name().unwrap(),
            "00033-0050-08-26-manual.json"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn id_1_does_not_match_index_10() {
        let dir = scratch("no-false-match");
        touch(&dir, "00010-0000-10-21-timed.json");
        assert!(matches!(
            resolve("1", &dir, "json"),
            Err(ResolveError::NotFound { .. })
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn all_zero_index_resolves_as_zero() {
        let dir = scratch("zero");
        touch(&dir, "00000-0000-10-21-timed.json");
        assert_eq!(
            resolve("0", &dir, "json").unwrap().file_name().unwrap(),
            "00000-0000-10-21-timed.json"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn extension_gates_the_glob() {
        let dir = scratch("ext");
        touch(&dir, "00001-timed.txt");
        touch(&dir, "index.jsonl");
        // Asking for json finds nothing (the .txt and .jsonl don't count).
        assert!(matches!(
            resolve("1", &dir, "json"),
            Err(ResolveError::NotFound { .. })
        ));
        // Asking for txt finds the capture.
        assert_eq!(
            resolve("1", &dir, "txt").unwrap().file_name().unwrap(),
            "00001-timed.txt"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn ambiguous_glob_errors_with_candidates() {
        let dir = scratch("ambig");
        touch(&dir, "00007-a-timed.json");
        touch(&dir, "00007-b-manual.json");
        match resolve("7", &dir, "json") {
            Err(ResolveError::Ambiguous { matches, .. }) => {
                assert_eq!(matches.len(), 2);
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dir_slash_id_takes_precedence_over_glob() {
        let dir = scratch("dir-id");
        touch(&dir, "temp.json");
        touch(&dir, "00001-timed.json");
        // Exact `dir/temp.json` wins; not a numeric id, so the glob wouldn't fire
        // anyway, but this pins the dir/id case.
        assert_eq!(
            resolve("temp.json", &dir, "json")
                .unwrap()
                .file_name()
                .unwrap(),
            "temp.json"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn explicit_path_resolves_and_beats_dir() {
        let dir = scratch("path");
        let other = scratch("path-other");
        touch(&other, "elsewhere.json");
        let p = other.join("elsewhere.json");
        // A real path is honored even though `dir` is a different directory.
        assert_eq!(resolve(p.to_str().unwrap(), &dir, "json").unwrap(), p);
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&other);
    }
}
