/// Avoid plain `color_eyre::install()`; it overwrites `logging::init()`'s panic hook.
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
