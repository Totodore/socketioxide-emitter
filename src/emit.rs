use std::fmt;

use crate::Driver;

/// An error that occurs when broadcasting messages.
pub enum EmitError<D: Driver> {
    /// The underlying driver error.
    Driver(D::Error),
    /// A parsing error that is specific to the parser used.
    Parser(socketioxide_core::parser::ParserError),
}
impl<D: Driver> fmt::Debug for EmitError<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitError::Driver(err) => write!(f, "Driver error: {}", err),
            EmitError::Parser(err) => write!(f, "Serialization error: {}", err),
        }
    }
}
impl<D: Driver> fmt::Display for EmitError<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
impl<D: Driver> std::error::Error for EmitError<D> {}

/// The available socket.io parsers when encoding messages.
/// Ensure that all your socket.io systems use the same parser.
#[derive(Debug, Clone, Copy, Default)]
pub enum Parser {
    /// Specify the [common socket.io parser](https://docs.rs/socketioxide-parser-common/latest/socketioxide_parser_common/).
    /// This is the default parser for all socket.io systems.
    #[cfg(feature = "common-parser")]
    #[cfg_attr(feature = "common-parser", default)]
    Common,
    /// Specify the [msgpack socket.io parser](https://docs.rs/socketioxide-parser-msgpack/latest/socketioxide_parser_msgpack/).
    /// If you choose to use this parser, ensure that all your socket.io systems support msgpack.
    #[cfg(feature = "msgpack-parser")]
    #[cfg_attr(
        all(feature = "msgpack-parser", not(feature = "common-parser")),
        default
    )]
    MsgPack,
}
