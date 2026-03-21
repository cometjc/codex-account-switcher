mod app;
mod input;
mod model;
mod render;

use std::path::PathBuf;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let snapshot_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .context("usage: plot-viewer <snapshot-path>")?;

    let snapshot = model::PlotSnapshot::load_from_path(&snapshot_path)?;
    app::run(snapshot)
}
