use bytes::Bytes;
use http::header::HeaderValue;
use std::fmt;
use std::error::Error;
use std::hash::Hash;

/// A possible error when converting a `MetadataValue` from a string or byte
/// slice.
#[derive(Debug)]
pub struct InvalidMetadataValue {
    _priv: (),
}

mod value_encoding {
    use bytes::Bytes;
    use http::header::HeaderValue;
    use std::fmt;
    use super::InvalidMetadataValueBytes;

    pub trait Sealed {
        #[doc(hidden)]
        fn is_empty(value: &[u8]) -> bool;

        #[doc(hidden)]
        fn from_bytes(value: &[u8]) -> Result<HeaderValue, InvalidMetadataValueBytes>;

        #[doc(hidden)]
        fn from_shared(value: Bytes) -> Result<HeaderValue, InvalidMetadataValueBytes>;

        #[doc(hidden)]
        fn from_static(value: &'static str) -> HeaderValue;

        #[doc(hidden)]
        fn decode(value: &[u8]) -> Result<Bytes, InvalidMetadataValueBytes>;

        #[doc(hidden)]
        fn equals(a: &HeaderValue, b: &[u8]) -> bool;

        #[doc(hidden)]
        fn values_equal(a: &HeaderValue, b: &HeaderValue) -> bool;

        #[doc(hidden)]
        fn fmt(value: &HeaderValue, f: &mut fmt::Formatter) -> fmt::Result;
    }
}

pub trait ValueEncoding: Clone + Eq + PartialEq + Hash + self::value_encoding::Sealed {
    /// Returns true if the provided key is valid for this ValueEncoding type.
    /// For example, `Ascii::is_valid_key("a") == true`,
    /// `Ascii::is_valid_key("a-bin") == false`.
    fn is_valid_key(key: &str) -> bool;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Ascii {}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Binary {}

// ===== impl ValueEncoding =====

impl self::value_encoding::Sealed for Ascii {
    fn is_empty(value: &[u8]) -> bool {
        value.is_empty()
    }

    fn from_bytes(value: &[u8]) -> Result<HeaderValue, InvalidMetadataValueBytes> {
        HeaderValue::from_bytes(value)
            .map_err(|_| { InvalidMetadataValueBytes::new() })
    }

    fn from_shared(value: Bytes) -> Result<HeaderValue, InvalidMetadataValueBytes> {
       HeaderValue::from_shared(value)
            .map_err(|_| {
                InvalidMetadataValueBytes::new()
            })
    }

    fn from_static(value: &'static str) -> HeaderValue {
        HeaderValue::from_static(value)
    }

    fn decode(value: &[u8]) -> Result<Bytes, InvalidMetadataValueBytes> {
        Ok(Bytes::from(value))
    }

    fn equals(a: &HeaderValue, b: &[u8]) -> bool {
        a.as_bytes() == b
    }

    fn values_equal(a: &HeaderValue, b: &HeaderValue) -> bool {
        a == b
    }

    fn fmt(value: &HeaderValue, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(value, f)
    }
}

impl ValueEncoding for Ascii {
    fn is_valid_key(key: &str) -> bool {
        !Binary::is_valid_key(key)
    }
}

impl self::value_encoding::Sealed for Binary {
    fn is_empty(value: &[u8]) -> bool {
        for c in value {
            if *c != b'=' {
                return false;
            }
        }
        true
    }

    fn from_bytes(value: &[u8]) -> Result<HeaderValue, InvalidMetadataValueBytes> {
        let encoded_value : String = base64::encode_config(value, base64::STANDARD_NO_PAD);
        HeaderValue::from_shared(encoded_value.into())
            .map_err(|_| { InvalidMetadataValueBytes::new() })
    }

    fn from_shared(value: Bytes) -> Result<HeaderValue, InvalidMetadataValueBytes> {
        Self::from_bytes(value.as_ref())
    }

    fn from_static(value: &'static str) -> HeaderValue {
        if !base64::decode(value).is_ok() {
            panic!("Invalid base64 passed to from_static: {}", value);
        }
        unsafe {
            // Because this is valid base64 this must be a valid HTTP header value,
            // no need to check again by calling from_shared.
            HeaderValue::from_shared_unchecked(Bytes::from_static(value.as_ref()))
        }
    }

    fn decode(value: &[u8]) -> Result<Bytes, InvalidMetadataValueBytes> {
        base64::decode(value)
            .map(|bytes_vec| bytes_vec.into())
            .map_err(|_| { InvalidMetadataValueBytes::new() })
    }

    fn equals(a: &HeaderValue, b: &[u8]) -> bool {
        if let Ok(decoded) = base64::decode(a.as_bytes()) {
            decoded == b
        } else {
            a.as_bytes() == b
        }
    }

    fn values_equal(a: &HeaderValue, b: &HeaderValue) -> bool {
        let decoded_a = Self::decode(a.as_bytes());
        let decoded_b = Self::decode(b.as_bytes());
        if decoded_a.is_ok() && decoded_b.is_ok() {
            decoded_a.unwrap() == decoded_b.unwrap()
        } else {
            !decoded_a.is_ok() && !decoded_b.is_ok()
        }
    }

    fn fmt(value: &HeaderValue, f: &mut fmt::Formatter) -> fmt::Result {
        if let Ok(decoded) = Self::decode(value.as_bytes()) {
            write!(f, "{:?}", decoded)
        } else {
            write!(f, "b[invalid]{:?}", value)
        }
    }
}

impl ValueEncoding for Binary {
    fn is_valid_key(key: &str) -> bool {
        key.ends_with("-bin")
    }
}

// ===== impl InvalidMetadataValue =====

impl InvalidMetadataValue {
    pub(crate) fn new() -> Self {
        InvalidMetadataValue { _priv: () }
    }
}

impl fmt::Display for InvalidMetadataValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.description().fmt(f)
    }
}

impl Error for InvalidMetadataValue {
    fn description(&self) -> &str {
        "failed to parse metadata value"
    }
}

/// A possible error when converting a `MetadataValue` from a string or byte
/// slice.
#[derive(Debug)]
pub struct InvalidMetadataValueBytes(InvalidMetadataValue);

// ===== impl InvalidMetadataValueBytes =====

impl InvalidMetadataValueBytes {
    pub(crate) fn new() -> Self {
        InvalidMetadataValueBytes(InvalidMetadataValue::new())
    }
}

impl fmt::Display for InvalidMetadataValueBytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for InvalidMetadataValueBytes {
    fn description(&self) -> &str {
        self.0.description()
    }
}
