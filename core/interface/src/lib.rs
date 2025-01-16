use context::Context;
use types::{Error, Event, File, IdentityPlugin, Rsx};

#[allow(unused_variables)]
#[plugy::macros::plugin]
// TODO: Make the plugin api more composable
// TODO: Handle types like File and Event better
pub trait Plugin: Send + Sync + 'static {
    fn on_load(&self, ctx: &mut Context) -> Result<(), Error>;

    fn process_file(&self, ctx: &mut Context, file: File) -> Result<File, Error> {
        Ok(file)
    }

    fn on_event(&self, ctx: &Context, ev: Event) -> Result<(), Error> {
        Ok(())
    }

    fn on_unload(&self, ctx: &mut Context) -> Result<(), Error> {
        Ok(())
    }

    fn render(&self, ctx: &Context, view: Event) -> Result<Rsx, Error> {
        Err(Error::InvalidRender)
    }
}

impl Plugin for IdentityPlugin {
    fn on_load(&self, _ctx: &mut Context) -> Result<(), Error> {
        unreachable!()
    }
}
