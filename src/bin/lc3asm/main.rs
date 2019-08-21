use std::env;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "lc3asm", about = "LC-3 assembly assembler")]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    /// Output file, <filename_of_input>.asm if not present
    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>,
    /// Enable backtrace(RUSTC_BACKTRACE=1). Convenience option for debugging.
    #[structopt(short = "b", long = "backtrace")]
    backtrace: bool,
    /// Show parsed structure before assembling
    #[structopt(short = "s", long = "structure")]
    print_pairs: bool,
}

fn main() -> Result<(), lc3asm::Error> {
    let opt = Opt::from_args();
    if opt.backtrace {
        env::set_var("RUST_BACKTRACE", "1");
    }

    let raw_data = &fs::read(&opt.input)?;
    let input_str = opt.input.clone().into_os_string().into_string().unwrap();
    let pairs = lc3asm::parse(std::str::from_utf8(raw_data)?).map_err(|err| {
        eprintln!("Cannot parse {}\n{}", input_str, err);
        err
    })?;

    if opt.print_pairs {
        eprintln!("{:#?}", pairs)
    }

    let (assembled, symbol_table) = lc3asm::assemble_from_pairs(pairs.collect()).map_err(|err| {
        eprintln!("Cannot assemble {}\n{}", input_str, err);
        err
    })?;
    let mut default_out_path = opt.input;
    default_out_path.set_extension("obj");
    let obj_output_path = opt.output.unwrap_or(default_out_path);
    let sym_output_path = obj_output_path.clone();
    sym_output_path.set_extension("sym");
    fs::write(obj_output_path, assembled)?;
    fs::write(sym_output_path, symbol_table)?;
    Ok(())
}
