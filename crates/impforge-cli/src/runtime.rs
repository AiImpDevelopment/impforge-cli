// SPDX-License-Identifier: MIT
//! Runtime bootstrap — wires every crate into the Orchestrator as a Module.
//!
//! Default power mode after bootstrap is `DeepSleep` for every module,
//! so `impforge-cli --help` uses <5 MB of resident memory.  Commands
//! wake their required modules explicitly.

use impforge_emergence::Orchestrator;
use std::sync::Arc;

pub fn bootstrap_orchestrator() -> anyhow::Result<Orchestrator> {
    let orc = Orchestrator::new();

    orc.register(Arc::new(impforge_scaffold::Module_))?;
    orc.register(Arc::new(impforge_models::Module_))?;
    orc.register(Arc::new(impforge_mcp_server::Module_))?;
    orc.register(Arc::new(impforge_contribute::Module_))?;
    orc.register(Arc::new(impforge_export::Module_))?;
    orc.register(Arc::new(impforge_autonomy::Module_))?;
    orc.register(Arc::new(impforge_crown_jewel::Module_))?;
    orc.register(Arc::new(impforge_bench::Module_))?;
    orc.register(Arc::new(impforge_remote::Module_))?;
    orc.register(Arc::new(impforge_universal::Module_))?;

    Ok(orc)
}
