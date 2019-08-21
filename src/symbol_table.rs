use crate::Rule;
use pest::iterators::Pair;
use std::collections::BTreeMap;
use std::fmt::{Write, Error as FmtError};

const TABLE_HEADER: & str = r#"//Symbol Name		Page Address
//----------------	------------
"#;
const SPACES: & str = "                              ";

pub(crate) type SymbolTable<'i> = BTreeMap<String, (usize, Pair<'i, Rule>)>;

pub fn table_to_string(sym: SymbolTable<'_>, entry: usize) -> Result<String, FmtError> {
    let mut s = String::from(TABLE_HEADER);
    for (key, (idx, _)) in sym {
        let space_size = 28 - key.len() - 4;
        writeln!(s, "//\t{}{}{:04X}", key, &SPACES[0..space_size], entry + idx)?;
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    #[test]
    fn test_program1() -> Result<(), Error> {
        let parsed = crate::parse(
            r#"	.ORIG	x0400
	ST	R3, SAVE3
	ST	R2, SAVE2
	AND	R2, R2, #0
TEST	IN
	BRz	TEST
	ADD	R1, R0, #-10
	BRn	FINISH
	ADD	R1, R0, #-15
	NOT	R1, R1
	BRn FINISH
	HALT
FINISH	ADD	R2, R2, #1
	HALT
SAVE3	.FILL	x0000
SAVE2	.FILL	x0000
	.END"#,
        )?
        .collect::<Vec<_>>();
        let (symbols, size, entry) = crate::first_pass(&parsed)?;
        let table_str = table_to_string(symbols, entry)?;
        assert_eq!(
            table_str,
            r#"//Symbol Name		Page Address
//----------------	------------
//	FINISH                  040B
//	SAVE2                   040E
//	SAVE3                   040D
//	TEST                    0403
"#
        );
        Ok(())
    }
}
