mod codegen;
mod cubemx;

use anyhow::Result;
use cubemx::Db;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(about = "Code generation for the stm32f3xx-hal crate")]
enum Command {
    #[structopt(about = "Generate GPIO mappings from an STM32CubeMX database")]
    Gpio {
        #[structopt(parse(from_os_str), help = "Path of the STM32CubeMX MCU database")]
        db_path: PathBuf,

        fname: String,
    },
    #[structopt(about = "Generate F4-like DMA tables")]
    Dma {
        #[structopt(parse(from_os_str), help = "Path of the STM32CubeMX MCU database")]
        db_path: PathBuf,

        fname: String,
    },
}

fn main() -> Result<()> {
    match Command::from_args() {
        Command::Gpio { db_path, fname } => handle_gpio(db_path, &fname),
        Command::Dma { db_path, fname } => handle_dma(db_path, &fname),
    }
}

fn handle_gpio(db_path: PathBuf, fname: &str) -> Result<()> {
    let db = cubemx::Db::new(db_path);

    emit_autogen_comment(&db)?;

    let gpio_ips = cubemx::load_f3_gpio_ips(&db, fname)?;
    codegen::gpio::gen_mappings(&gpio_ips)?;

    Ok(())
}

fn handle_dma(db_path: PathBuf, fname: &str) -> Result<()> {
    let db = cubemx::Db::new(db_path);

    emit_autogen_comment(&db)?;

    let dma_maps: Result<_> = cubemx::load_f3_dma_ips(&db, fname)?
        .iter()
        .map(crate::codegen::dma::ip_to_table)
        .collect();
    crate::codegen::dma::print_table(&dma_maps?);
    Ok(())
}

fn emit_autogen_comment(db: &Db) -> Result<()> {
    let package = cubemx::package::load(db)?;
    codegen::gen_autogen_comment(&package);

    Ok(())
}
