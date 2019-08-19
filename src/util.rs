use bitstream_io::write::BitWriter;
use bitstream_io::{BigEndian, Numeric, SignedNumeric};
use pest::iterators::Pair;
use std::io::Result as IOResult;
use std::io::Write;
use std::num::ParseIntError;

pub fn parse_number_literal(s: &str) -> Result<i64, ParseIntError> {
    s.parse().or_else(|_| match s.chars().nth(0) {
        Some('#') => s[1..].parse(),
        Some('x') | Some('X') => i64::from_str_radix(&s[1..], 16),
        _ => panic!("Invalid decimal literal {} received", s),
    })
}

pub fn parse_register_literal(s: &str) -> Result<i64, ParseIntError> {
    match s.to_ascii_lowercase().chars().nth(0) {
        Some('r') => i64::from_str_radix(&s[1..], 16),
        _ => panic!("Invalid register literal {} received", s),
    }
}

pub enum PCOffsetTarget {
    Symbol(String),
    ExplicitOffset(i64),
}

pub fn parse_pc_pair(target_pair: &Pair<super::Rule>) -> PCOffsetTarget {
    let pair_str = target_pair.as_str();

    match pair_str.parse() {
        Ok(offset) => PCOffsetTarget::ExplicitOffset(offset), // TODO: Determine whether this is allowed
        Err(_) => match pair_str.chars().nth(0) {
            Some('#') => PCOffsetTarget::ExplicitOffset(pair_str[1..].parse().unwrap()),
            _ => PCOffsetTarget::Symbol(pair_str.to_owned()),
        },
    }
}

/// Wrapper struct for [BitWriter] which extends some functionality
/// e.g. counting total bytes written without consuming itself.
pub struct BitVecWriter<W>
where
    W: Write,
{
    wr: BitWriter<W, BigEndian>,
    bits_pushed: u32,
}

impl<W> BitVecWriter<W>
where
    W: Write,
{
    pub fn new(wr: W) -> Self {
        BitVecWriter {
            wr: BitWriter::new(wr),
            bits_pushed: 0,
        }
    }

    /// Wrapping method for [BitWriter::write]
    pub fn write<U>(&mut self, bits: u32, value: U) -> IOResult<()>
    where
        U: Numeric,
    {
        self.wr.write(bits, value).map(|_| {
            self.bits_pushed += bits;
        })
    }

    /// Wrapping method for [BitWriter::write]
    pub fn write_signed<S>(&mut self, bits: u32, value: S) -> IOResult<()>
    where
        S: SignedNumeric,
    {
        self.wr.write_signed(bits, value).map(|_| {
            self.bits_pushed += bits;
        })
    }

    /// Wrapping method for [BitWriter::write_bit]
    pub fn write_bit(&mut self, bit: bool) -> IOResult<()> {
        self.wr.write_bit(bit).map(|_| {
            self.bits_pushed += 1;
        })
    }

    /// Returns a pair of `(bytes_written, remaining_bits_count)`
    pub fn count_written(&self) -> (u32, u8) {
        (self.bits_pushed / 8, (self.bits_pushed % 8) as u8)
    }

    pub fn into_inner(self) -> BitWriter<W, BigEndian> {
        self.wr
    }
}

#[macro_export]
macro_rules! collect_inner {
    ($pair:expr) => {
        &$pair.into_inner().collect::<Vec<_>>()[..]
    };
}

#[macro_export]
macro_rules! pair_error_message {
    ($pair:expr, $($arg:tt)*) => {
        PestError::new_from_span(
            PestErrorVariant::<Rule>::CustomError {
                message: format!($($arg)*)
            },
            $pair.as_span()
        )
    };
}

#[macro_export]
macro_rules! write_fields {
    ($wr:expr) => {};

    ($wr:expr $(,[$($more:tt)+])*,) => {
        write_fields!($wr $(,[$($more)+])*);
    };

    ($wr:expr, [const; $bits:expr, $value:expr] $(,[$($more:tt)+])*) => {
        $wr.write($bits, $value)?;
        write_fields!($wr $(,[$($more)+])*);
    };

    ($wr:expr, [bool; $value:expr] $(,[$($more:tt)+])*) => {
        $wr.write_bit($value)?;
        write_fields!($wr $(,[$($more)+])*);
    };

    ($wr:expr, [number $name:ident; $bits:expr, $pair:expr] $(,[$($more:tt)+])*) => {
        let $name = util::parse_number_literal($pair.as_str())?;
        write_fields!($wr,
            [$name; $bits, $pair, $name]
            $(,[$($more)+])*,
        );
    };

    ($wr:expr, [number_signed $name:ident; $bits:expr, $pair:expr] $(,[$($more:tt)+])*) => {
        let $name = util::parse_number_literal($pair.as_str())?;
        write_fields!($wr,
            [signed $name; $bits, $pair, $name]
            $(,[$($more)+])*,
        );
    };

    ($wr:expr, [register $name:ident; $pair:expr] $(,[$($more:tt)+])*) => {
        let $name = util::parse_register_literal($pair.as_str())?;
        write_fields!($wr,
            [$name; 3, $pair, $name]
            $(,[$($more)+])*,
        );
    };

    ($wr:expr, [pcoffset; $bits:expr, $pair:expr, $table:expr] $(,[$($more:tt)+])*) => {
        let _current_offset = ($wr.count_written().0-2) / 2;
        match util::parse_pc_pair($pair) {
            util::PCOffsetTarget::Symbol(_sym) => {
                if let Some((_offset, _)) = $table.get(&_sym) {
                    write_fields!(
                        $wr,
                        [signed pc_offset; $bits, $pair, *_offset as i32 - _current_offset as i32 - 1],
                    );
                } else {
                    return Err(pair_error_message!(
                        $pair,
                        "Cannot find symbol {}, available symbols: {}",
                        _sym,
                        $table.keys().map(String::to_owned).collect::<Vec<_>>().join(", "),
                    ).into())
                }
            },
            util::PCOffsetTarget::ExplicitOffset(_offset) => {
                write_fields!($wr, [signed pc_offset; $bits, $pair, _offset]);
            }
        }
        write_fields!($wr $(,[$($more)+])*);
    };

    ($wr:expr, [$name:ident; $bits:expr, $pair:expr, $value:expr] $(,[$($more:tt)+])*) => {
        write_fields!($wr, [$name; $bits, $pair, $value, write] $(,[$($more)+])*);
    };

    ($wr:expr, [signed $name:ident; $bits:expr, $pair:expr, $value:expr] $(,[$($more:tt)+])*) => {
        write_fields!($wr, [$name; $bits, $pair, $value, write_signed] $(,[$($more)+])*);
    };

    ($wr:expr,
        [$name:ident; $bits:expr, $pair:expr, $value:expr, $func:ident]
        $(,[$($more:tt)+])*) => {
        $wr.$func($bits, $value).map_err(|e| -> Error {
            if e.kind() == IOErrorKind::InvalidInput {
                pair_error_message!(
                    $pair,
                    "Value {} overflows for given field {}",
                    $value,
                    stringify!($name)
                )
                .into()
            } else {
                e.into()
            }
        })?;
        write_fields!($wr $(,[$($more)+])*);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitvecwriter() {
        let mut buf = Vec::new();
        let mut bvw = BitVecWriter::new(&mut buf);
        bvw.write(8, 3).unwrap();
        assert_eq!(bvw.count_written(), (1, 0));
        bvw.write(3, 5).unwrap();
        assert_eq!(bvw.count_written(), (1, 3));
        bvw.write_bit(true).unwrap();
        bvw.write(4, 5).unwrap();
        assert_eq!(bvw.count_written(), (2, 0));
        assert_eq!(bvw.into_inner().into_writer(), &[3, 0b1011_0101]);
    }

    #[test]
    fn test_bvw_overflow() {
        let mut buf = Vec::new();
        let mut bvw = BitVecWriter::new(&mut buf);
        bvw.write(4, 16).unwrap_err();
    }
}
