#![allow(clippy::inconsistent_digit_grouping, clippy::unreadable_literal)]
pub use error::Error;
use pest::error::Error as PestError;
use pest::error::ErrorVariant as PestErrorVariant;
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use std::io::{ErrorKind as IOErrorKind, Write};
use unescape::unescape;

#[cfg(test)]
mod asm_tests;
mod error;
mod util;

/// Parser struct.
#[derive(Parser)]
#[grammar = "asm.pest"]
pub struct AsmParser;

/// Reads code from input and produces [Vec] of parsed pairs.
pub fn parse(input: &str) -> Result<Pairs<Rule>, PestError<Rule>> {
    AsmParser::parse(Rule::file, input)
}

/// Reads code from input and produces object code output.
pub fn assemble(input: impl AsRef<str>) -> Result<Vec<u8>, Error> {
    let asm_parsed = parse(input.as_ref())?.collect();
    assemble_from_pairs(asm_parsed)
}

/// Reads code from slice of pairs and produces object code output.
pub fn assemble_from_pairs(asm_parsed: Vec<Pair<Rule>>) -> Result<Vec<u8>, Error> {
    let (symbols, size) = first_pass(&asm_parsed)?; // TODO: export symbol table to file
    let buf: Vec<u8> = Vec::with_capacity(size);
    let mut wr = util::BitVecWriter::new(buf);

    for pair in asm_parsed {
        second_pass(pair, &mut wr, &symbols)?;
        assert_eq!(
            wr.count_written().1,
            0,
            "Each assembly pass should write aligned bytes to buffer"
        );
    }

    let buf = wr.into_inner().into_writer();
    assert_eq!(buf.len(), size * 2 + 2);

    Ok(buf)
}

type SymbolTable<'i> = HashMap<String, (usize, Pair<'i, Rule>)>;

fn first_pass<'i>(pairs: &[Pair<'i, Rule>]) -> Result<(SymbolTable<'i>, usize), Error> {
    let mut symbols: HashMap<String, (usize, Pair<Rule>)> = HashMap::new();
    let mut offset = 0;

    for pair in pairs {
        match pair.as_rule() {
            Rule::label_decl => {
                let name = pair.as_str();
                if let Some((_, prev_pair)) = symbols.get(name) {
                    return Err(pair_error_message!(
                        pair.clone(),
                        "Duplicate symbol definition\n{}",
                        pair_error_message!(
                            prev_pair.clone(),
                            "Note: First definition of the symbol was here",
                        )
                    )
                    .into());
                }
                symbols.insert(name.into(), (offset, pair.clone()));
            }
            Rule::instruction | Rule::trap_code | Rule::fill => offset += 1,
            Rule::blkw => {
                if let [content] = collect_inner!(pair.clone()) {
                    offset += util::parse_number_literal(content.as_str())? as usize;
                } else {
                    unreachable!(pair);
                }
            }
            Rule::stringz => {
                let string = pair
                    .clone()
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str();
                match unescape(string) {
                    Some(us) => {
                        offset += us.bytes().len() + 1;
                    }
                    None => {
                        return Err(
                            pair_error_message!(pair.clone(), "Invalid escape sequence",).into(),
                        );
                    }
                }
            }
            _ => (),
        }
    }

    Ok((symbols, offset))
}

fn second_pass<W: Write>(
    pair: Pair<Rule>,
    wr: &mut util::BitVecWriter<W>,
    symbols: &SymbolTable,
) -> Result<(), Error> {
    match pair.as_rule() {
        Rule::orig => {
            wr.write(16, util::parse_number_literal(pair.into_inner().as_str())?)?;
        }

        Rule::instruction => {
            for inner_pair in pair.into_inner() {
                second_pass(inner_pair, wr, symbols)?;
            }
        }

        rule @ Rule::add | rule @ Rule::and => {
            if let [dr, sr1, sr2] = collect_inner!(pair) {
                write_fields!(
                    wr,
                    [const; 4, if rule == Rule::add { 0b0001 } else { 0b0101 }],
                    [register destination_register; dr],
                    [register source_register1; sr1],
                    [const; 3, 0b000],
                    [register source_register2; sr2],
                );
            } else {
                unreachable!();
            }
        }

        rule @ Rule::add_immd | rule @ Rule::and_immd => {
            if let [dr, sr1, immd5] = collect_inner!(pair) {
                write_fields!(
                    wr,
                    [const; 4, if rule == Rule::add_immd { 0b0001 } else { 0b0101 }],
                    [register destination_register; dr],
                    [register source_register1; sr1],
                    [bool; true],
                    [number_signed immediate; 5, immd5],
                );
            } else {
                unreachable!();
            }
        }

        Rule::not => {
            if let [dr, sr] = collect_inner!(pair) {
                write_fields!(
                    wr,
                    [const; 4, 0b1001],
                    [register destination_register; dr],
                    [register source_register; sr],
                    [const; 6, 0b111111]
                );
            } else {
                unreachable!();
            }
        }

        Rule::br => {
            if let [br_option, label] = collect_inner!(pair) {
                let br_option = br_option.as_str();
                let (n, z, p) = (
                    br_option.find('n').is_some(),
                    br_option.find('z').is_some(),
                    br_option.find('p').is_some(),
                );
                let implicit_unconditional_branch = (n, z, p) == (false, false, false);
                if implicit_unconditional_branch {
                    // TODO: Print position
                    eprintln!("Warning: Use BRnzp instead of BR for clarity");
                }
                write_fields!(
                    wr,
                    [const; 4, 0b0000],
                    [bool; n||implicit_unconditional_branch],
                    [bool; z||implicit_unconditional_branch],
                    [bool; p||implicit_unconditional_branch],
                    [pcoffset; 9, label, symbols],
                );
            } else {
                unreachable!();
            }
        }

        Rule::jmp => {
            if let [br] = collect_inner!(pair) {
                write_fields!(
                    wr,
                    [const; 7, 0b1100_000],
                    [register base_register; br],
                    [const; 6, 0b000000]
                );
            } else {
                unreachable!();
            }
        }

        Rule::jsr => {
            if let [pcoffset] = collect_inner!(pair) {
                write_fields!(
                    wr,
                    [const; 5, 0b0100_1],
                    [pcoffset; 11, pcoffset, symbols],
                );
            } else {
                unreachable!();
            }
        }

        Rule::jsrr => {
            if let [br] = collect_inner!(pair) {
                write_fields!(
                    wr,
                    [const; 7, 0b0100_000],
                    [register base_register; br],
                    [const; 6, 0b000000],
                );
            } else {
                unreachable!();
            }
        }

        rule @ Rule::ld
        | rule @ Rule::ldi
        | rule @ Rule::lea
        | rule @ Rule::st
        | rule @ Rule::sti => {
            if let [dosr, label] = collect_inner!(pair) {
                write_fields!(
                    wr,
                    [const; 4, match rule {
                        Rule::ld => 0b0010,
                        Rule::ldi => 0b1010,
                        Rule::lea => 0b1110,
                        Rule::st => 0b0011,
                        Rule::sti => 0b1011,
                        _ => unreachable!()
                    }],
                    [register destination_or_source_register; dosr],
                    [pcoffset; 9, label, symbols],
                );
            } else {
                unreachable!();
            }
        }

        rule @ Rule::ldr | rule @ Rule::str => {
            if let [dosr, br, offset_] = collect_inner!(pair) {
                write_fields!(
                    wr,
                    [const; 4, if rule == Rule::ldr { 0b0110 } else { 0b0111 }],
                    [register destination_or_source_register; dosr],
                    [register base_register; br],
                    [number_signed offset; 6, offset_],
                );
            } else {
                unreachable!();
            }
        }

        Rule::ret => {
            write_fields!(wr, [const; 16, 0b1100_000_111_000000]);
        }

        Rule::rti => {
            write_fields!(wr, [const; 16, 0b1000_0000_0000_0000]);
        }

        Rule::trap => {
            if let [trap_vect] = collect_inner!(pair) {
                write_fields!(
                    wr,
                    [const; 8, 0b1111_0000],
                    [number_signed trap_vector; 8, trap_vect],
                );
            } else {
                unreachable!();
            }
        }

        Rule::trap_code => {
            write_fields!(
            wr,
            [const; 8, 0b1111_0000],
            [const; 8,
                match pair.as_str().to_lowercase().trim() {
                    "getc" => 0x20,
                    "out" => 0x21,
                    "puts" => 0x22,
                    "in" => 0x23,
                    "putsp" => 0x24,
                    "halt" => 0x25,
                    _ => unreachable!(),
                }
            ],
            );
        }

        Rule::stringz => {
            unescape(
                pair.into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str(),
            )
            .unwrap()
            .as_bytes()
            .iter()
            .chain(&[0])
            .cloned()
            .map(u16::from)
            .map(|x| wr.write(16, x))
            .collect::<Result<(), _>>()?;
        }

        Rule::nop => {
            write_fields!(
                wr,
                [const; 16, 0b0000_0000_0000_0000],
            );
        }

        Rule::blkw => {
            if let [blocks] = collect_inner!(pair) {
                for _ in 0..util::parse_number_literal(blocks.as_str()).unwrap() {
                    write_fields!(wr, [const; 16, 0u16]);
                }
            } else {
                unreachable!();
            }
        }

        Rule::fill => {
            if let [content] = collect_inner!(pair) {
                write_fields!(wr, [number_signed fill_content; 16, content]);
            } else {
                unreachable!();
            }
        }

        Rule::end => (),
        Rule::EOI => (),
        Rule::label_decl => (),
        _ => unreachable!("{:#?}", pair),
    }
    Ok(())
}
