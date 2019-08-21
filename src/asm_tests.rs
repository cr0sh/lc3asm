use super::*;
use console::Term;
use lazy_static::lazy_static;
use lc3::vm::VM;
use lc3dbg::helper::view_mem_entry;
use lc3dbg::symbol::TableEntry;

lazy_static! {
    static ref TEMPORARY_SYMBOL_TABLE: Vec<TableEntry> = vec![TableEntry::Unknown; 1 << 16];
}

macro_rules! asm_test {
    ($(#[$ignore:meta])? $name:ident, $code:literal, $input:literal, $output:literal
        $(,insert $in_addr:literal <- $in_value:literal)*
        $(,assert $out_addr:literal == $out_value:literal)* $(,)?) => {
        #[test]
        $(#[$ignore])?
        fn $name() -> Result<(), Error> {
            #![allow(unused_variables, unused_assignments, unused_mut)]
            let mut vm = VM::new();
            let obj = assemble($code)?.0;
            vm.load_u8(obj.as_ref());

            asm_test!(@insert_mem_values vm $(,$in_addr <- $in_value)*);

            eprintln!("");
            let stderr = Term::stderr();
            for delta in 0..(obj.len() / 2) {
                view_mem_entry(vm.pc as usize + delta, &vm, &TEMPORARY_SYMBOL_TABLE, &stderr)?;
            }

            let mut input_buf = $input.as_bytes();
            let mut output_buf: Vec<u8> = Vec::new();
            vm.run(&mut input_buf, &mut output_buf);

            assert_eq!(
                String::from_utf8_lossy(&output_buf),
                concat!($output, "\n\n--- halting the LC-3 ---\n\n")
            );

            let mut counter: usize = 1;

            asm_test!(@assert_mem_values vm, counter $(,$out_addr == $out_value)*);
            Ok(())
        }
    };

    (@insert_mem_values $vm:expr) => {};

    (@insert_mem_values $vm:expr, $addr:literal <- $value:literal $(,$($more:tt)+)*) => {
        $vm.mem[$addr] = $value;
        asm_test!(@insert_mem_values $vm $(,$($more)+)*);
    };

    (@assert_mem_values $vm:expr, $counter:expr) => {};

    (@assert_mem_values $vm:expr, $counter:expr, $addr:literal == $value:literal $(,$($more:tt)+)*) => {
        assert_eq!(
            $vm.mem[$addr],
            $value,
            "Memory test #{} failure",
            $counter,
        );
        $counter += 1;
        asm_test!(@assert_mem_values $vm, $counter $(,$($more)+)*);
    };
}

asm_test!(
    adder,
    r#"
.ORIG   x3000
        AND     r0, r0, #0
        ADD     r0, r0, #8
        ADD     r0, r0, #8
        OUT
        HALT
.END
    "#,
    "",
    "\u{10}",
);

asm_test!(
    string_print,
    r#"
.ORIG   x3000
        LEA     r0, STZ
        PUTS
        HALT
STZ     .STRINGZ "HELLO\n"
.END
    "#,
    "",
    "HELLO\n",
);

asm_test!(
    interger_division,
    r#"
.ORIG	x3000
        LDI	R0, NUM		; R0 = NUM
        LDI	R1, DIV
        NOT	R1, R1
        ADD	R1, R1, #1	; R1 = -DIV
        AND	R2, R2, #0	; R2 = 0 (count)
LOOP	ADD	R3, R0, R1
        BRn	OUTL		; DIVISION COMPLETE!
        ADD	R2, R2, #1
        ADD	R0, R3, #0
        BR	LOOP
OUTL	STI	R0, REM
        STI	R2, QTT
        HALT

NUM	    .FILL x4000
DIV	    .FILL x4001
QTT	    .FILL x4002
REM	    .FILL x4003
.END
    "#,
    "",
    "",
    insert 0x4000 <- 7,
    insert 0x4001 <- 3,
    assert 0x4002 == 2,
    assert 0x4003 == 1,
);
