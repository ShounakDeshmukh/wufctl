/// Installs color-eyre's report hook and a panic hook that restores the
/// terminal before printing.
///
/// This can't be the plain `color_eyre::install()` one-liner: that installs
/// its own panic hook, and only one `std::panic::set_hook` can be active at
/// a time - it would silently replace `logging::init()`'s intent of also
/// recording the panic to the log file. `HookBuilder` lets us build both
/// hooks without installing color-eyre's panic hook directly, so we can
/// wrap it: restore the terminal, log the raw message via `tracing`, then
/// delegate to color-eyre's formatter for the pretty on-screen report.
pub fn install() -> color_eyre::Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default().into_hooks();
    eyre_hook.install()?;

    let panic_hook = panic_hook.into_panic_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        tracing::error!("{info}");
        panic_hook(info);
    }));

    Ok(())
}
