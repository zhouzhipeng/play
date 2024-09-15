use async_channel::Receiver;
use async_trait::async_trait;
use play_shared::tpl_engine_api::{Template, TemplateData, TplEngineAPI};
use include_dir::Dir;
use include_dir::include_dir;
pub static TEMPLATES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");


pub struct FakeTplEngine{

}
#[async_trait]
impl TplEngineAPI for FakeTplEngine{
    async fn run_loop(&self, req_receiver: Receiver<TemplateData>) {
        loop{
            if let Ok(data)=  req_receiver.recv().await {
                data.response.send(match data.template {
                    Template::StaticTemplate { content, .. } => content.to_string(),
                    Template::DynamicTemplate { content, .. } => content.to_string(),
                    Template::PythonCode { content, .. } => content.to_string(),
                }).await;
            }
        }
    }
}