#[path = "../models/mod.rs"]
mod models;

use anyhow::Result;
use const_format::formatcp;
use serde_reflection::{Registry, Tracer, TracerConfig};
use std::{env, path::PathBuf};

use self::models::pin::PinValidationResult;

const MODULE_NAME: &str = "core_domain";
const DART_OUTPUT_PATH: &str = formatcp!("{}/../../wallet_app/pub/{}", env!("CARGO_MANIFEST_DIR"), MODULE_NAME);

fn main() -> Result<()> {
    let mut tracer = Tracer::new(TracerConfig::default());
    tracer.trace_simple_type::<PinValidationResult>().unwrap();
    let registry = tracer.registry().unwrap();

    generate_dart(&registry)
}

// Create Dart class definitions.
fn generate_dart(registry: &Registry) -> Result<()> {
    let config = serde_generate::CodeGeneratorConfig::new(MODULE_NAME.to_string())
        .with_encodings(vec![serde_generate::Encoding::Bincode])
        .with_c_style_enums(true);
    let generator = serde_generate::dart::CodeGenerator::new(&config);
    generator.output(PathBuf::from(DART_OUTPUT_PATH), registry)?;

    Ok(())
}
