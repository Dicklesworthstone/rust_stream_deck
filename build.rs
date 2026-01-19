use vergen_gix::{BuildBuilder, CargoBuilder, Emitter, GixBuilder, RustcBuilder, SysinfoBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build = BuildBuilder::default().build_timestamp(true).build()?;
    let cargo = CargoBuilder::default()
        .target_triple(true)
        .opt_level(true)
        .build()?;
    let gix = GixBuilder::default()
        .sha(true)
        .commit_timestamp(true)
        .dirty(true)
        .build()?;
    let rustc = RustcBuilder::default().semver(true).channel(true).build()?;
    let si = SysinfoBuilder::default().build()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&gix)?
        .add_instructions(&rustc)?
        .add_instructions(&si)?
        .emit()?;

    Ok(())
}
