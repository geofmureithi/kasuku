use context::Context;
use interface::Plugin;
use plugy::macros::plugin_impl;
use types::Error;

pub struct TemplatePlugin;

#[plugin_impl]
impl Plugin for TemplatePlugin {
    fn on_load(&self, _ctx: &mut Context) -> Result<(), Error> {
        // Create table templates
        // Register setting menu item "Templates"
        // Subscribe to plugin events
        // CreateTemplate -> creates a template given the path and context

        // registerAction/Command

        // Automator: Every Morning -> InvokeCommand: Generate Daily Note

        Ok(())
    }
}
