# lc3asm
```
LC-3 assembly code parser & assembler
Copyright (C) 2019 Nam Jeonghyun. (ska827@snu.ac.kr)
```

## Installation
`cargo install lc3asm`

## Assembly language parser
`lc3asm::AsmParser` and `lc3asm::Rule` provides an assembly parser and rules. Parser grammar follows definitions
from [Introduction to Computing Systems: From Bits and Gates to C and Beyond](https://www.amazon.com/Introduction-Computing-Systems-Gates-Beyond/dp/0072467509). Plus, some features are added:

 - UTF-8 string literal support(currently <u>panics</u> if non UTF-8 file is given)
 - Backslash escape sequence(`"\\"`/`"\r"`/`"\n"`/`"\t"`/`"\b"`/`"\f"`/`"\u00A9"`) support in string literal
   - Note that unicode escape sequence requires exactly four hexadecimal numbers for each character.
 - Elegant syntax error reporting(powered by [Pest](https://pest.rs))
 - For compatiability issues, decimal literal without `#` is accepted for immediate values,
  but this could be removed in the future.
