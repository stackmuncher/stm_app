encoding_rs is a Gecko-oriented Free Software / Open Source implementation of the Encoding Standard in Rust. Gecko-oriented means that converting to and from UTF-16 is supported in addition to converting to and from UTF-8, that the performance and streamability goals are browser-oriented, and that FFI-friendliness is a goal.

Additionally, the mem module provides functions that are useful for applications that need to be able to deal with legacy in-memory representations of Unicode.

For expectation setting, please be sure to read the sections UTF-16LE, UTF-16BE and Unicode Encoding Schemes, ISO-8859-1 and Web / Browser Focus below.

There is a long-form write-up about the design and internals of the crate.