use std::panic;

use color_eyre::Result;
use color_eyre::config::HookBuilder;

/// Avoid plain `color_eyre::install()`; it overwrites `logging::init()`'s panic hook.
pub fn install() -> Result<()> {
    let (panic_hook, eyre_hook) = HookBuilder::default().into_hooks();
    eyre_hook.install()?;

    let panic_hook = panic_hook.into_panic_hook();
    panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        tracing::error!("{info}");
        panic_hook(info);
    }));

    Ok(())
}
