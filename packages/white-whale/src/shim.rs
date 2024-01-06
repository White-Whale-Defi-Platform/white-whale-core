// osmosis-std-derive needs a `::crate::shim` to work
// we rip this file from `osmosis-std` package:
// https://github.com/osmosis-labs/osmosis-rust/blob/main/packages/osmosis-std/src/shim.rs

#[derive(Clone, PartialEq, Eq, ::prost::Message, schemars::JsonSchema)]
pub struct Any {
    /// A URL/resource name that uniquely identifies the type of the serialized
    /// protocol buffer message. This string must contain at least
    /// one "/" character. The last segment of the URL's path must represent
    /// the fully qualified name of the type (as in
    /// `path/google.protobuf.Duration`). The name should be in a canonical form
    /// (e.g., leading "." is not accepted).
    ///
    /// In practice, teams usually precompile into the binary all types that they
    /// expect it to use in the context of Any. However, for URLs which use the
    /// scheme `http`, `https`, or no scheme, one can optionally set up a type
    /// server that maps type URLs to message definitions as follows:
    ///
    /// * If no scheme is provided, `https` is assumed.
    /// * An HTTP GET on the URL must yield a \[google.protobuf.Type][\]
    ///   value in binary format, or produce an error.
    /// * Applications are allowed to cache lookup results based on the
    ///   URL, or have them precompiled into a binary to avoid any
    ///   lookup. Therefore, binary compatibility needs to be preserved
    ///   on changes to types. (Use versioned type names to manage
    ///   breaking changes.)
    ///
    /// Note: this functionality is not currently available in the official
    /// protobuf release, and it is not used for type URLs beginning with
    /// type.googleapis.com.
    ///
    /// Schemes other than `http`, `https` (or the empty scheme) might be
    /// used with implementation specific semantics.
    ///
    #[prost(string, tag = "1")]
    pub type_url: ::prost::alloc::string::String,
    /// Must be a valid serialized protocol buffer of the above specified type.
    #[prost(bytes = "vec", tag = "2")]
    pub value: ::prost::alloc::vec::Vec<u8>,
}

macro_rules! impl_prost_types_exact_conversion {
    ($t:ident | $($arg:ident),*) => {
        impl From<$t> for prost_types::$t {
            fn from(src: $t) -> Self {
                prost_types::$t {
                    $(
                        $arg: src.$arg,
                    )*
                }
            }
        }

        impl From<prost_types::$t> for $t {
            fn from(src: prost_types::$t) -> Self {
                $t {
                    $(
                        $arg: src.$arg,
                    )*
                }
            }
        }
    };
}

impl_prost_types_exact_conversion! { Any | type_url, value }