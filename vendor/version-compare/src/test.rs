use crate::Cmp;

/// Struct containing a version number with some meta data.
/// Such a set can be used for testing.
///
/// # Arguments
///
/// - `0`: The version string.
/// - `1`: Number of version parts.
pub struct Version(pub &'static str, pub usize);

/// List of version numbers with metadata for dynamic tests
pub const VERSIONS: &'static [Version] = &[
    Version("1", 1),
    Version("1.2", 2),
    Version("1.2.3.4", 4),
    Version("1.2.3.4.5.6.7.8", 8),
    Version("0", 1),
    Version("0.0.0", 3),
    Version("1.0.0", 3),
    Version("0.0.1", 3),
    Version("", 0),
    Version(".", 0),
    Version("...", 0),
    Version("1.2.dev", 3),
    Version("1.2-dev", 3),
    Version("1.2.alpha.4", 4),
    Version("1.2-alpha-4", 4),
    Version("snapshot.1.2", 3),
    Version("snapshot-1.2", 3),
    // Issue: https://github.com/timvisee/version-compare/issues/26
    Version("0.0.1-test.0222426166a", 6),
    Version("0.0.1-test.0222426166565421816516584651684351354", 5),
    Version("0.0.1-test.02224261665a", 5),
    Version("0.0.1-test.02224261665d7b1b689816d12f6bcacb", 5),
];

/// List of version numbers that contain errors with metadata for dynamic tests
pub const VERSIONS_ERROR: &'static [Version] = &[
    Version("abc", 1),
    Version("alpha.dev.snapshot", 3),
    Version("test. .snapshot", 3),
];

/// Struct containing two version numbers, and the comparison operator.
/// Such a set can be used for testing.
///
/// # Arguments
///
/// - `0`: The main version.
/// - `1`: The other version.
/// - `2`: The comparison operator.
pub struct VersionCombi(pub &'static str, pub &'static str, pub Cmp);

/// List of version combinations for dynamic tests
pub const COMBIS: &'static [VersionCombi] = &[
    VersionCombi("1", "1", Cmp::Eq),
    VersionCombi("1.0.0.0", "1", Cmp::Eq),
    VersionCombi("1", "1.0.0.0", Cmp::Eq),
    VersionCombi("0", "0", Cmp::Eq),
    VersionCombi("0.0.0", "0", Cmp::Eq),
    VersionCombi("0", "0.0.0", Cmp::Eq),
    VersionCombi("", "", Cmp::Eq),
    VersionCombi("", "0.0", Cmp::Eq),
    VersionCombi("0.0", "", Cmp::Eq),
    VersionCombi("", "0.1", Cmp::Lt),
    VersionCombi("0.1", "", Cmp::Gt),
    VersionCombi("1.2.3", "1.2.3", Cmp::Eq),
    VersionCombi("1.2.3", "1.2.4", Cmp::Lt),
    VersionCombi("1.0.0.1", "1.0.0.0", Cmp::Gt),
    VersionCombi("1.0.0.0", "1.0.0.1", Cmp::Lt),
    VersionCombi("1.2.3.4", "1.2", Cmp::Gt),
    VersionCombi("1.2", "1.2.3.4", Cmp::Lt),
    VersionCombi("1.2.3.4", "2", Cmp::Lt),
    VersionCombi("2", "1.2.3.4", Cmp::Gt),
    VersionCombi("123", "123", Cmp::Eq),
    VersionCombi("123", "1.2.3", Cmp::Gt),
    VersionCombi("1.2.3", "123", Cmp::Lt),
    VersionCombi("1.1.2", "1.1.30-dev", Cmp::Lt),
    VersionCombi("1.2.3", "1.2.3.alpha", Cmp::Gt),
    VersionCombi("1.2.3", "1.2.3-dev", Cmp::Gt),
    VersionCombi("1.2.3 RC0", "1.2.3 rc1", Cmp::Lt),
    VersionCombi("1.2.3 rc2", "1.2.3 RC99", Cmp::Lt),
    VersionCombi("1.2.3 RC3", "1.2.3 RC1", Cmp::Gt),
    VersionCombi("1.2.3a", "1.2.3b", Cmp::Lt),
    VersionCombi("1.2.3b", "1.2.3a", Cmp::Gt),
    VersionCombi("1.2.3.dev", "1.2.3.alpha", Cmp::Gt),
    VersionCombi("1.2.3-dev", "1.2.3-alpha", Cmp::Gt),
    VersionCombi("1.2.3.dev.1", "1.2.3.alpha", Cmp::Gt),
    VersionCombi("1.2.3-dev-1", "1.2.3-alpha", Cmp::Gt),
    VersionCombi("version-compare 3.2.0 / build 0932", "3.2.5", Cmp::Lt),
    VersionCombi("version-compare 3.2.0 / build 0932", "3.1.1", Cmp::Gt),
    VersionCombi(
        "version-compare 1.4.1 / build 0043",
        "version-compare 1.4.1 / build 0043",
        Cmp::Eq,
    ),
    VersionCombi(
        "version-compare 1.4.1 / build 0042",
        "version-compare 1.4.1 / build 0043",
        Cmp::Lt,
    ),
    // Issue: https://github.com/timvisee/version-compare/issues/24
    VersionCombi("7.2p1", "7.1", Cmp::Gt),
    // TODO: inspect these cases
    VersionCombi("snapshot.1.2.3", "1.2.3.alpha", Cmp::Lt),
    VersionCombi("snapshot-1.2.3", "1.2.3-alpha", Cmp::Lt),
];

/// List of invalid version combinations for dynamic tests
pub const COMBIS_ERROR: &'static [VersionCombi] = &[
    VersionCombi("1.2.3", "1.2.3", Cmp::Lt),
    VersionCombi("1.2", "1.2.0.0", Cmp::Ne),
    VersionCombi("1.2.3.dev", "dev", Cmp::Eq),
    VersionCombi("snapshot", "1", Cmp::Lt),
];
