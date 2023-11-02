#[cfg(unix)]
type RawFd = std::os::fd::RawFd;
#[cfg(not(unix))]
type RawFd = std::convert::Infallible;

/// Error type for `from_env_ext` function.
#[derive(Debug)]
pub struct FromEnvError {
    pub(crate) inner: FromEnvErrorInner,
}

/// Kind of an error returned from `from_env_ext` function.
#[derive(Debug)]
#[non_exhaustive]
pub enum FromEnvErrorKind {
    /// There is no environment variable that describes jobserver to inherit.
    NoEnvVar,
    /// There is no jobserver in the environment variable.
    /// Variables associated with Make can be used for passing data other than jobserver info.
    NoJobserver,
    /// Cannot parse jobserver environment variable value, incorrect format.
    CannotParse,
    /// Cannot open path or name from the jobserver environment variable value.
    CannotOpenPath,
    /// Cannot open file descriptor from the jobserver environment variable value.
    CannotOpenFd,
    /// File descriptor from the jobserver environment variable value is not a pipe.
    NotAPipe,
    /// Jobserver inheritance is not supported on this platform.
    Unsupported,
}

impl FromEnvError {
    /// Get the error kind.
    pub fn kind(&self) -> FromEnvErrorKind {
        match self.inner {
            FromEnvErrorInner::NoEnvVar => FromEnvErrorKind::NoEnvVar,
            FromEnvErrorInner::NoJobserver => FromEnvErrorKind::NoJobserver,
            FromEnvErrorInner::CannotParse(_) => FromEnvErrorKind::CannotParse,
            FromEnvErrorInner::CannotOpenPath(..) => FromEnvErrorKind::CannotOpenPath,
            FromEnvErrorInner::CannotOpenFd(..) => FromEnvErrorKind::CannotOpenFd,
            FromEnvErrorInner::NotAPipe(..) => FromEnvErrorKind::NotAPipe,
            FromEnvErrorInner::Unsupported => FromEnvErrorKind::Unsupported,
        }
    }
}

impl std::fmt::Display for FromEnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            FromEnvErrorInner::NoEnvVar => write!(f, "there is no environment variable that describes jobserver to inherit"),
            FromEnvErrorInner::NoJobserver => write!(f, "there is no `--jobserver-fds=` or `--jobserver-auth=` in the environment variable"),
            FromEnvErrorInner::CannotParse(s) => write!(f, "cannot parse jobserver environment variable value: {s}"),
            FromEnvErrorInner::CannotOpenPath(s, err) => write!(f, "cannot open path or name {s} from the jobserver environment variable value: {err}"),
            FromEnvErrorInner::CannotOpenFd(fd, err) => write!(f, "cannot open file descriptor {fd} from the jobserver environment variable value: {err}"),
            FromEnvErrorInner::NotAPipe(fd, None) => write!(f, "file descriptor {fd} from the jobserver environment variable value is not a pipe"),
            FromEnvErrorInner::NotAPipe(fd, Some(err)) => write!(f, "file descriptor {fd} from the jobserver environment variable value is not a pipe: {err}"),
            FromEnvErrorInner::Unsupported => write!(f, "jobserver inheritance is not supported on this platform"),
        }
    }
}
impl std::error::Error for FromEnvError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            FromEnvErrorInner::CannotOpenPath(_, err) => Some(err),
            FromEnvErrorInner::NotAPipe(_, Some(err)) | FromEnvErrorInner::CannotOpenFd(_, err) => {
                Some(err)
            }
            _ => None,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum FromEnvErrorInner {
    NoEnvVar,
    NoJobserver,
    CannotParse(String),
    CannotOpenPath(String, std::io::Error),
    CannotOpenFd(RawFd, std::io::Error),
    NotAPipe(RawFd, Option<std::io::Error>),
    Unsupported,
}
