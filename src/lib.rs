mod ffi;

use std::ffi::CStr;
use std::ffi::CString;
use std::fmt;
use std::os::raw::c_uint;
use std::ptr;

pub fn sample_rate() -> i32 {
    unsafe { ffi::pv_sample_rate() }
}

pub fn frame_length() -> i32 {
    unsafe { ffi::pv_cobra_frame_length() }
}

pub fn lib_version() -> &'static str {
    let cstr = unsafe { CStr::from_ptr(ffi::pv_cobra_version()) };
    cstr.to_str().unwrap()
}

#[derive(Debug, Clone)]
pub enum Error {
    NullValue,
    OutOfMemory,
    IoError,
    InvalidArgument,
    StopIteration,
    KeyError,
    InvalidState,
    RuntimeError,
    ActivationError,
    ActivationLimitReached,
    ActivationThrottled,
    ActivationRefused,
    UnknownError(c_uint),
}

impl From<ffi::pv_status_t> for Error {
    fn from(status: ffi::pv_status_t) -> Self {
        match status {
            ffi::pv_status_t_PV_STATUS_OUT_OF_MEMORY => Error::OutOfMemory,
            ffi::pv_status_t_PV_STATUS_IO_ERROR => Error::IoError,
            ffi::pv_status_t_PV_STATUS_INVALID_ARGUMENT => Error::InvalidArgument,
            ffi::pv_status_t_PV_STATUS_STOP_ITERATION => Error::StopIteration,
            ffi::pv_status_t_PV_STATUS_KEY_ERROR => Error::KeyError,
            ffi::pv_status_t_PV_STATUS_INVALID_STATE => Error::InvalidState,
            ffi::pv_status_t_PV_STATUS_RUNTIME_ERROR => Error::RuntimeError,
            ffi::pv_status_t_PV_STATUS_ACTIVATION_ERROR => Error::ActivationError,
            ffi::pv_status_t_PV_STATUS_ACTIVATION_LIMIT_REACHED => Error::ActivationLimitReached,
            ffi::pv_status_t_PV_STATUS_ACTIVATION_THROTTLED => Error::ActivationThrottled,
            ffi::pv_status_t_PV_STATUS_ACTIVATION_REFUSED => Error::ActivationRefused,
            _ => Error::UnknownError(status as c_uint),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NullValue => write!(f, "unexpected null value"),
            Error::OutOfMemory => write!(f, "out of memory"),
            Error::IoError => write!(f, "I/O error"),
            Error::InvalidArgument => write!(f, "invalid argument"),
            Error::StopIteration => write!(f, "stop iteration"),
            Error::KeyError => write!(f, "key error"),
            Error::InvalidState => write!(f, "invalid state"),
            Error::RuntimeError => write!(f, "runtime error"),
            Error::ActivationError => write!(f, "activation error"),
            Error::ActivationLimitReached => write!(f, "activation limit reached"),
            Error::ActivationThrottled => write!(f, "activation throttled"),
            Error::ActivationRefused => write!(f, "activation refused"),
            Error::UnknownError(c) => write!(f, "non-zero status returned: {}", c),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

pub struct Cobra {
    cobra: *mut ffi::pv_cobra,
}

impl Cobra {
    pub fn new<S: Into<Vec<u8>>>(access_key: S) -> Result<Self, Error> {
        let access_key = CString::new(access_key).map_err(|_err| Error::NullValue)?;
        let mut cobra: *mut ffi::pv_cobra = ptr::null_mut();
        let status = unsafe { ffi::pv_cobra_init(access_key.as_ptr(), &mut cobra) };
        if status != 0 {
            Err(Error::from(status))
        } else if cobra.is_null() {
            Err(Error::NullValue)
        } else {
            Ok(Cobra { cobra })
        }
    }

    pub fn process(&mut self, pcm: &[i16]) -> Result<f32, Error> {
        let mut confidence: f32 = 0.0;
        let status = unsafe { ffi::pv_cobra_process(self.cobra, pcm.as_ptr(), &mut confidence) };
        if status != 0 {
            Err(Error::from(status))
        } else {
            Ok(confidence)
        }
    }
}

impl Drop for Cobra {
    fn drop(&mut self) {
        unsafe {
            ffi::pv_cobra_delete(self.cobra);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_sample_rate() {
        // Just make sure it's callable
        sample_rate();
    }

    #[test]
    fn check_frame_length() {
        // Just make sure it's callable
        frame_length();
    }

    #[test]
    fn check_lib_version() {
        // Just make sure it's callable
        lib_version();
    }
}
