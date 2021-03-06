// Copyright (c) 2017-present PyO3 Project and Contributors

use crate::err::{PyErr, PyResult};
use crate::ffi;
use crate::instance::{Py, PyObjectWithGIL};
use crate::object::PyObject;
use crate::python::{Python, ToPyPointer};
use crate::types::exceptions;
use crate::types::PyObjectRef;
use std::borrow::Cow;
use std::os::raw::c_char;
use std::{mem, str};

/// Represents a Python `string`.
#[repr(transparent)]
pub struct PyString(PyObject);

pyobject_native_type!(PyString, ffi::PyUnicode_Type, ffi::PyUnicode_Check);

/// Represents a Python `byte` string.
#[repr(transparent)]
pub struct PyBytes(PyObject);

pyobject_native_type!(PyBytes, ffi::PyBytes_Type, ffi::PyBytes_Check);

impl PyString {
    /// Creates a new Python string object.
    ///
    /// Panics if out of memory.
    pub fn new(_py: Python, s: &str) -> Py<PyString> {
        let ptr = s.as_ptr() as *const c_char;
        let len = s.len() as ffi::Py_ssize_t;
        unsafe { Py::from_owned_ptr_or_panic(ffi::PyUnicode_FromStringAndSize(ptr, len)) }
    }

    pub fn from_object<'p>(
        src: &'p PyObjectRef,
        encoding: &str,
        errors: &str,
    ) -> PyResult<&'p PyString> {
        unsafe {
            src.py()
                .from_owned_ptr_or_err::<PyString>(ffi::PyUnicode_FromEncodedObject(
                    src.as_ptr(),
                    encoding.as_ptr() as *const c_char,
                    errors.as_ptr() as *const c_char,
                ))
        }
    }

    /// Get the Python string as a byte slice.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let mut size: ffi::Py_ssize_t = mem::uninitialized();
            let data = ffi::PyUnicode_AsUTF8AndSize(self.0.as_ptr(), &mut size) as *const u8;
            // PyUnicode_AsUTF8AndSize would return null if the pointer did not reference a valid
            // unicode object, but because we have a valid PyString, assume success
            debug_assert!(!data.is_null());
            std::slice::from_raw_parts(data, size as usize)
        }
    }

    /// Convert the `PyString` into a Rust string.
    ///
    /// Returns a `UnicodeDecodeError` if the input is not valid unicode
    /// (containing unpaired surrogates).
    pub fn to_string(&self) -> PyResult<Cow<str>> {
        match std::str::from_utf8(self.as_bytes()) {
            Ok(s) => Ok(Cow::Borrowed(s)),
            Err(e) => Err(PyErr::from_instance(
                exceptions::UnicodeDecodeError::new_utf8(self.py(), self.as_bytes(), e)?,
            )),
        }
    }

    /// Convert the `PyString` into a Rust string.
    ///
    /// Unpaired surrogates invalid UTF-8 sequences are
    /// replaced with U+FFFD REPLACEMENT CHARACTER.
    pub fn to_string_lossy(&self) -> Cow<str> {
        String::from_utf8_lossy(self.as_bytes())
    }
}

impl PyBytes {
    /// Creates a new Python byte string object.
    /// The byte string is initialized by copying the data from the `&[u8]`.
    ///
    /// Panics if out of memory.
    pub fn new(_py: Python, s: &[u8]) -> Py<PyBytes> {
        let ptr = s.as_ptr() as *const c_char;
        let len = s.len() as ffi::Py_ssize_t;
        unsafe { Py::from_owned_ptr_or_panic(ffi::PyBytes_FromStringAndSize(ptr, len)) }
    }

    /// Creates a new Python byte string object from raw pointer.
    ///
    /// Panics if out of memory.
    pub unsafe fn from_ptr(_py: Python, ptr: *const u8, len: usize) -> Py<PyBytes> {
        Py::from_owned_ptr_or_panic(ffi::PyBytes_FromStringAndSize(
            ptr as *const _,
            len as isize,
        ))
    }

    /// Get the Python string as a byte slice.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let buffer = ffi::PyBytes_AsString(self.as_ptr()) as *const u8;
            let length = ffi::PyBytes_Size(self.as_ptr()) as usize;
            debug_assert!(!buffer.is_null());
            std::slice::from_raw_parts(buffer, length)
        }
    }
}

#[cfg(test)]
mod test {
    use super::PyString;
    use crate::conversion::{FromPyObject, PyTryFrom, ToPyObject};
    use crate::instance::AsPyRef;
    use crate::object::PyObject;
    use crate::python::Python;
    use std::borrow::Cow;

    #[test]
    fn test_non_bmp() {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let s = "\u{1F30F}";
        let py_string = s.to_object(py);
        assert_eq!(s, py_string.extract::<String>(py).unwrap());
    }

    #[test]
    fn test_extract_str() {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let s = "Hello Python";
        let py_string = s.to_object(py);

        let s2: &str = FromPyObject::extract(py_string.as_ref(py).into()).unwrap();
        assert_eq!(s, s2);
    }

    #[test]
    fn test_as_bytes() {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let s = "ascii 🐈";
        let obj: PyObject = PyString::new(py, s).into();
        let py_string = <PyString as PyTryFrom>::try_from(obj.as_ref(py)).unwrap();
        assert_eq!(s.as_bytes(), py_string.as_bytes());
    }

    #[test]
    fn test_to_string_ascii() {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let s = "ascii";
        let obj: PyObject = PyString::new(py, s).into();
        let py_string = <PyString as PyTryFrom>::try_from(obj.as_ref(py)).unwrap();
        assert!(py_string.to_string().is_ok());
        assert_eq!(Cow::Borrowed(s), py_string.to_string().unwrap());
    }

    #[test]
    fn test_to_string_unicode() {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let s = "哈哈🐈";
        let obj: PyObject = PyString::new(py, s).into();
        let py_string = <PyString as PyTryFrom>::try_from(obj.as_ref(py)).unwrap();
        assert!(py_string.to_string().is_ok());
        assert_eq!(Cow::Borrowed(s), py_string.to_string().unwrap());
    }
}
