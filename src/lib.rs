mod ffi;

use std::ffi::CStr;
use std::ffi::CString;
use std::fmt;
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
    NonZeroStatus(u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NullValue => write!(f, "unexpected null value"),
            Error::NonZeroStatus(s) => write!(f, "non-zero status returned: {}", s),
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
            Err(Error::NonZeroStatus(status))
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
            Err(Error::NonZeroStatus(status))
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
