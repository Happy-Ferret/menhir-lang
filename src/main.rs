extern crate llvm_sys as llvm;
extern crate libc;
extern crate itertools;
#[macro_use]
extern crate clap;
extern crate uuid;


mod ast;
mod compileerror;
mod bytecode;
mod parser;
mod typechecker;
mod span;
mod llvmbackend;
mod target;

use std::path::{Path, PathBuf};
use std::process::exit;
use clap::ArgMatches;
use parser::{ParserOptions, parse_file};
use typechecker::{type_check_module};
use bytecode::{compile_to_byte_code, optimize_module, ByteCodeModule, OptimizationLevel};
use compileerror::{CompileResult, CompileError};
use llvmbackend::{CodeGenOptions, llvm_code_generation, llvm_init, link};
use target::Target;


fn default_output_file(input_file: &str) -> String
{
    Path::new(&input_file)
        .file_stem()
        .expect("Invalid input file")
        .to_str()
        .expect("Invalid input file")
        .into()
}

fn dump_byte_code(bc_mod: &ByteCodeModule, dump_flags: &str)
{
    if dump_flags.contains("bytecode") || dump_flags.contains("all") {
        println!("bytecode:");
        println!("------\n");
        println!("{}", bc_mod);
        println!("------\n");
    }
}

fn parse(parser_options: &ParserOptions, input_file: &str, dump_flags: &str, optimize: bool, target: &Target) -> CompileResult<ByteCodeModule>
{
    let mut module = parse_file(parser_options, input_file, target)?;
    type_check_module(&mut module, target)?;

    if dump_flags.contains("ast") || dump_flags.contains("all") {
        println!("AST:");
        println!("------\n");
        use ast::TreePrinter;
        module.print(0);
        println!("------\n");
    }

    let mut bc_mod = compile_to_byte_code(&module, target)?;
    if optimize {
        optimize_module(&mut bc_mod, OptimizationLevel::Normal);
    } else {
        optimize_module(&mut bc_mod, OptimizationLevel::Minimal);
    }

    dump_byte_code(&bc_mod, dump_flags);
    Ok(bc_mod)
}

fn build_command(matches: &ArgMatches, dump_flags: &str) -> CompileResult<i32>
{
    let input_file = matches.value_of("INPUT_FILE").expect("No input file given");
    let optimize = matches.is_present("OPTIMIZE");
    let output_file = matches.value_of("OUTPUT_FILE").map(|v| v.to_string()).unwrap_or_else(|| default_output_file(input_file));
    let target_machine = llvm_init()?;

    let parser_options = ParserOptions{
        import_dirs: matches.value_of("IMPORTS")
            .map(|dirs| dirs.split(',').map(PathBuf::from).collect())
            .unwrap_or_else(Vec::new),
    };

    let bc_mod = parse(&parser_options, input_file, dump_flags, optimize, &target_machine.target)?;
    let opts = CodeGenOptions{
        dump_ir: dump_flags.contains("ir") || dump_flags.contains("all"),
        build_dir: "build".into(),
        program_name: output_file.into(),
        optimize: optimize,
    };

    let ctx = llvm_code_generation(&bc_mod, &target_machine).map_err(CompileError::Other)?;
    link(&ctx, &opts)?;
    Ok(0)
}

fn run() -> CompileResult<i32>
{
    let app = clap_app!(cobrac =>
        (version: "0.1")
        (author: "Joris Guisson <joris.guisson@gmail.com>")
        (about: "Nomad language compiler")
        (@arg DUMP: -d --dump +takes_value "Dump internal compiler state for debug purposes. Argument can be all, ast, bytecode or ir. A comma separated list of these values is also supported.")
        (@subcommand build =>
            (about: "Build a menhir file")
            (version: "0.1")
            (@arg INPUT_FILE: +required "File to build")
            (@arg OUTPUT_FILE: -o --output +takes_value "Name of binary to create (by default input file without the extensions)")
            (@arg OPTIMIZE: -O --optimize "Optimize the code")
            (@arg IMPORTS: -I --imports +takes_value "Directory to look for imports, use a comma separated list for more then one.")
        )
    );

    let matches = app.get_matches();
    let dump_flags = matches.value_of("DUMP").unwrap_or("");

    if let Some(build_matches) = matches.subcommand_matches("build") {
        build_command(build_matches, dump_flags)
    } else {
        println!("{}", matches.usage());
        Ok(1)
    }

}

fn main()
{
    match run()
    {
        Ok(ret) => exit(ret),
        Err(e) => {
            e.print();
            exit(-1);
        },
    }
}
